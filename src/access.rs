//! Storj DCS Access Grant and bound types.

use crate::helpers;
use crate::EncryptionKey;
use crate::Ensurer;
use crate::Error;

use std::ffi::{CStr, CString};
use std::time::Duration;
use std::vec::Vec;

use uplink_sys as ulksys;

/// Represents an Access Grant
///
/// An Access Grant contains everything to access a project and specific
/// buckets.
///
/// It includes a potentially-restricted API Key, a potentially-restricted set
/// of encryption information, and information about the Satellite responsible
/// for the project's metadata.
pub struct Access {
    /// The access type of the underlying c-bindings Rust crate that an instance
    /// of this struct represents and guard its life time until this instance
    /// drops.
    /// It's an access result because it's the one that holds the access grants
    /// and allows to free its memory.
    inner: ulksys::UplinkAccessResult,
}

impl Access {
    /// Creates a new Access from a serialized access grant string.
    pub fn new(saccess: &str) -> Result<Self, Error> {
        let saccess = match helpers::cstring_from_str_fn_arg("saccess", saccess) {
            Ok(cs) => cs,
            Err(e) => return Err(e),
        };

        let accres;
        // SAFETY: we trust that the underlying c-binding is safe, nonetheless
        // we ensure accres is correct through the ensure method of the
        // implemented Ensurer trait.
        unsafe {
            accres = *ulksys::uplink_parse_access(saccess.into_raw()).ensure();
        }

        if let Some(e) = Error::new_uplink(accres.error) {
            return Err(e);
        }

        Ok(Access { inner: accres })
    }

    /// generates a new access grant using a passphrase requesting to the
    /// Satellite a project-based salt for deterministic key derivation.
    pub fn request_access_with_passphrase(
        satellite_addr: &str,
        api_key: &str,
        passphrase: &str,
    ) -> Result<Self, Error> {
        let satellite_addr =
            match helpers::cstring_from_str_fn_arg("sattellite_addr", satellite_addr) {
                Ok(cs) => cs,
                Err(e) => return Err(e),
            };
        let api_key = match helpers::cstring_from_str_fn_arg("api_key", api_key) {
            Ok(cs) => cs,
            Err(e) => return Err(e),
        };
        let passphrase = match helpers::cstring_from_str_fn_arg("passphrase", passphrase) {
            Ok(cs) => cs,
            Err(e) => return Err(e),
        };

        let accres;
        // SAFETY: we trust that the underlying c-binding is safe, nonetheless
        // we ensure accres is correct through the ensure method of the
        // implemented Ensurer trait.
        unsafe {
            accres = *ulksys::uplink_request_access_with_passphrase(
                satellite_addr.into_raw(),
                api_key.into_raw(),
                passphrase.into_raw(),
            )
            .ensure();
        }

        if let Some(e) = Error::new_uplink(accres.error) {
            return Err(e);
        }

        Ok(Access { inner: accres })
    }

    /// overrides the root encryption key for the prefix in bucket with the
    /// encryption key.
    /// This method is useful for overriding the encryption key in user-specific
    /// access grants when implementing multitenancy in a single app bucket.
    /// See relevant information in the general crate documentation.
    pub fn override_encryption_key(
        &self,
        bucket: &str,
        prefix: &str,
        encryption_key: &EncryptionKey,
    ) -> Result<(), Error> {
        let bucket = match helpers::cstring_from_str_fn_arg("bucket", bucket) {
            Ok(cs) => cs,
            Err(e) => return Err(e),
        };

        let prefix = match helpers::cstring_from_str_fn_arg("prefix", prefix) {
            Ok(cs) => cs,
            Err(e) => return Err(e),
        };

        let err;
        // SAFETY: we trust that the underlying c-binding is safe.
        unsafe {
            err = ulksys::uplink_access_override_encryption_key(
                self.inner.access,
                bucket.into_raw(),
                prefix.into_raw(),
                encryption_key.to_uplink_c(),
            );
        }

        match Error::new_uplink(err) {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// It returns the satellite node URL associated with this access grant.
    pub fn satellite_address(&self) -> Result<&str, Error> {
        let strres;
        // SAFETY: we trust that the underlying c-binding is safe, nonetheless
        // we ensure strres is correct through the ensure method of the
        // implemented Ensurer trait.
        unsafe {
            strres = *ulksys::uplink_access_satellite_address(self.inner.access).ensure();
        }

        if let Some(e) = Error::new_uplink(strres.error) {
            return Err(e);
        }

        let addrres;
        // SAFETY: at this point we have already checked that strres.string is
        // NOT NULL.
        unsafe {
            addrres = CStr::from_ptr(strres.string).to_str();
        }

        Ok(addrres.expect("invalid underlying c-binding"))
    }

    /// It serializes an access grant such that it can be used to create a
    /// [`Self::new()`] instance of this type or parsed with other tools.
    pub fn serialize(&self) -> Result<&str, Error> {
        let strres;
        // SAFETY: we trust that the underlying c-binding is safe, nonetheless
        // we ensure strres is correct through the ensure method of the
        // implemented Ensurer trait.
        unsafe {
            strres = *ulksys::uplink_access_serialize(self.inner.access).ensure();
        }

        if let Some(e) = Error::new_uplink(strres.error) {
            return Err(e);
        }

        let serialized;
        // SAFETY: at this point we have already checked that strres.string is
        // NOT NULL.
        unsafe {
            serialized = CStr::from_ptr(strres.string).to_str();
        }

        Ok(serialized.expect("invalid underlying c-binding"))
    }

    /// It creates a new access grant with specific permissions.
    ///
    /// An access grant can only have their existing permissions restricted, and
    /// the resulting access will only allow for the intersection of all
    /// previous share calls in the access construction chain.
    ///
    /// Prefixes restrict the access grant (and internal encryption information)
    /// to only contain enough information to allow access to just those
    /// prefixes.
    ///
    /// To revoke an access grant see [`Project.revoke_access()`](struct.Project.html#method.revoke_access).
    ///
    pub fn share(
        &self,
        permission: &Permission,
        prefixes: Vec<SharePrefix>,
    ) -> Result<Access, Error> {
        let mut ulk_prefixes: Vec<ulksys::UplinkSharePrefix> = Vec::with_capacity(prefixes.len());

        for sp in prefixes {
            ulk_prefixes.push(sp.to_uplink_c())
        }

        let accres;
        // SAFETY: we trust that the underlying c-binding is safe, nonetheless
        // we ensure accres is correct through the ensure method of the
        // implemented Ensurer trait.
        unsafe {
            accres = *ulksys::uplink_access_share(
                self.inner.access,
                permission.to_uplink_c(),
                ulk_prefixes.as_mut_ptr(),
                ulk_prefixes.len() as i64,
            )
            .ensure()
        }

        if let Some(e) = Error::new_uplink(accres.error) {
            return Err(e);
        }

        Ok(Access { inner: accres })
    }
}

impl Drop for Access {
    fn drop(&mut self) {
        // SAFETY: we trust that the underlying c-binding is safe freeing the
        // memory of a correct UplinkAccessResult value.
        unsafe { ulksys::uplink_free_access_result(self.inner) }
    }
}

/// Represents a prefix to be shared.
#[derive(Debug)]
pub struct SharePrefix<'a> {
    bucket: &'a str,
    prefix: &'a str,
}

impl<'a> SharePrefix<'a> {
    /// Create a new prefix to be shared in the specified bucket.
    /// It returns an error if bucket or prefix contains a null character
    /// (0 byte).
    pub fn new(bucket: &'a str, prefix: &'a str) -> Result<Self, Error> {
        if bucket.contains('\0') {
            return Err(Error::new_invalid_arguments(
                "bucket",
                "cannot contains null bytes (0 byte)",
            ));
        }

        if prefix.contains('\0') {
            return Err(Error::new_invalid_arguments(
                "prefix",
                "cannot contains null bytes (0 byte)",
            ));
        }

        Ok(SharePrefix { bucket, prefix })
    }

    /// Returns the bucket where the prefix to be shared belongs.
    pub fn bucket(&self) -> &str {
        self.bucket
    }

    /// Returns the actual prefix to be shared.
    pub fn prefix(&self) -> &str {
        self.prefix
    }

    /// Returns an UplinkSharePrefix with the values of this SharedPrefix for
    /// interoperating with the uplink c-bindings.
    /// It panics if creating a CString from bucket or prefix returns an error
    /// because, in that case, there is a bug in the implementation or an
    /// internal misuage of this type.
    fn to_uplink_c(&self) -> ulksys::UplinkSharePrefix {
        let bucket = match CString::new(self.bucket) {
            Ok(cs) => cs,
            Err(e) => panic!(
                "BUG: Never set a value to the `bucket` field without previously guarantee there won't be an error converting it to a CString value. Error details: {}", e),
        };

        let prefix = match CString::new(self.prefix) {
            Ok(cs) => cs,
            Err(e) => panic!("BUG: Never set a value to the `prefix` field without previously guarantee there won't be an error converting it to a CString value. Error details: {}",e),
        };

        ulksys::UplinkSharePrefix {
            bucket: bucket.into_raw(),
            prefix: prefix.into_raw(),
        }
    }
}

/// Defines what actions and an optional specific period of time are granted to
/// a shared Access Grant.
/// A shared Access Grant can never has more permission that its parent, hence
/// even some allowed permission is set for the shared Access Grant but not to
/// its parent, the shared Access Grant won't be allowed.
/// shared Access Grant wont
/// See [`Access.share()`](struct.Access.html#method.share).
#[derive(Default)]
pub struct Permission {
    /// Gives permission to download the content of the objects and their
    /// associated metadata, but it does not allow listing buckets.
    pub allow_download: bool,
    /// Gives permission to create buckets and upload new objects. It does not
    /// allow overwriting existing objects unless allow_delete is granted too.
    pub allow_upload: bool,
    /// Gives permission to list buckets and getting the metadata of the
    /// objects. It does not allow downloading the content of the objects.
    pub allow_list: bool,
    /// Gives permission to delete buckets and objects. Unless either allow
    /// allow_download or allow_list is grated too, neither the metadata of the
    /// objects nor error information will be returned for deleted objects.
    pub allow_delete: bool,
    /// Restricts when the resulting access grant is valid for. If it is set
    /// then it must always be before not_after and the resulting access grant
    /// will not work if the satellite believes the time is before the set it
    /// one.
    /// The time is measured with the number of seconds since the Unix Epoch
    /// time.
    not_before: Option<Duration>,
    /// Restricts when the resulting access grant is valid for. If it is set
    /// then it must always be after not_before and the resulting access grant
    /// will not work if the satellite believes the time is after the set it
    /// one.
    /// The time is measured with the number of seconds since the Unix Epoch
    /// time.
    not_after: Option<Duration>,
}

impl Permission {
    /// Creates a permission that doesn't allow any operation, which is the
    /// default permission.
    /// This constructor is useful for creating a permission for after setting
    /// the specific allowed operations when none of the other constructors
    /// creates a permission with a set of allowed operations that works for
    /// your use case.
    pub fn new() -> Permission {
        Permission {
            ..Default::default()
        }
    }

    /// Creates a permission that allows all the operations (i.e. Downloading,
    /// uploading, listing and deleting).
    pub fn full() -> Permission {
        Permission {
            allow_download: true,
            allow_upload: true,
            allow_list: true,
            allow_delete: true,
            not_before: None,
            not_after: None,
        }
    }

    /// Creates a permission that allows for reading (i.e. Downloading) and
    /// listing.
    pub fn read_only() -> Permission {
        Permission {
            allow_download: true,
            allow_upload: false,
            allow_list: true,
            allow_delete: false,
            not_before: None,
            not_after: None,
        }
    }

    /// Creates a permission that allows for writing (i.e. Uploading) and
    /// deleting.
    pub fn write_only() -> Permission {
        Permission {
            allow_download: false,
            allow_upload: true,
            allow_list: false,
            allow_delete: true,
            not_before: None,
            not_after: None,
        }
    }

    /// Returns the duration from Unix Epoch time since this permission is
    /// valid.
    /// Return None when there is not before restriction.
    pub fn not_before(&self) -> Option<Duration> {
        self.not_before
    }

    /// Set a not before valid time for this permission or removing it when None
    /// is passed.
    /// An error is returned if since is more recent or equal to the current
    /// not after valid time of the permission, when not after is set.
    /// The time is measured with the number of seconds since the Unix Epoch
    /// time.
    pub fn set_not_before(&mut self, since: Option<Duration>) -> Result<(), Error> {
        if let Some(since) = since {
            if let Some(until) = self.not_after {
                if since >= until {
                    return Err(Error::new_invalid_arguments(
                    "since",
                    "cannot be more recent or equal to the not after valid time of the permission",
                ));
                }
            }
        }

        self.not_before = since;
        Ok(())
    }

    /// Returns the duration from Unix Epoch time until this permission is
    /// valid.
    /// Return None when there is not after restriction.
    pub fn not_after(&self) -> Option<Duration> {
        self.not_after
    }

    /// Set a not after valid time for this permission or removing it when None
    /// is passed.
    /// An error is returned if until is previous or equal to the current
    /// not before valid time of the permission, when not before is set.
    /// The time is measured with the number of seconds since the Unix Epoch
    /// time.
    pub fn set_not_after(&mut self, until: Option<Duration>) -> Result<(), Error> {
        if let Some(until) = until {
            if let Some(since) = self.not_before {
                if until <= since {
                    return Err(Error::new_invalid_arguments(
                    "until",
                    "cannot be previous or equal to the not before valid time of the permission",
                ));
                }
            }
        }

        self.not_after = until;
        Ok(())
    }

    /// Returns an UplinkPermission with the values of this Permission for
    /// interoperating with the uplink c-bindings.
    fn to_uplink_c(&self) -> ulksys::UplinkPermission {
        ulksys::UplinkPermission {
            allow_download: self.allow_download,
            allow_upload: self.allow_upload,
            allow_list: self.allow_list,
            allow_delete: self.allow_delete,
            not_before: self.not_before.map_or(0, |d| d.as_secs()) as i64,
            not_after: self.not_after.map_or(0, |d| d.as_secs()) as i64,
        }
    }
}

impl Ensurer for ulksys::UplinkAccessResult {
    fn ensure(&self) -> &Self {
        assert!(!self.access.is_null() || !self.error.is_null(), "invalid underlying c-binding returned UplinkAccessResult, access and error fields are both NULL");
        assert!(!self.access.is_null() && !self.error.is_null(), "invalid underlying c-binding returned UplinkAccessResult, access and error fields are both NOT NULL");
        self
    }
}

impl Ensurer for ulksys::UplinkStringResult {
    fn ensure(&self) -> &Self {
        assert!(!self.string.is_null() || !self.error.is_null(), "invalid underlying c-binding returned UplinkStringResult, string and error fields are both NULL");
        assert!(!self.string.is_null() && !self.error.is_null(), "invalid underlying c-binding returned UplinkStringResult, string and error fields are both NOT NULL");
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::error;

    #[test]
    fn test_share_prefix() {
        {
            // Pass a valid bucket and prefix.
            let sp = SharePrefix::new("a-bucket", "a/b/c")
                .expect("new shouldn't fail when passing a valid bucket and prefix");
            assert_eq!(sp.bucket(), "a-bucket", "bucket");
            assert_eq!(sp.prefix(), "a/b/c", "prefix");
        }

        {
            // Pass an invalid bucket.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                SharePrefix::new("a\0bucket\0", "a/b/c")
                    .expect_err("new passing a bucket with NULL bytes")
            {
                assert_eq!(names, "bucket", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte)",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }

        {
            // Pass an invalid prefix.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                SharePrefix::new("a-bucket", "a/b\0/c")
                    .expect_err("new passing a prefix with NULL bytes")
            {
                assert_eq!(names, "prefix", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte)",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }

        {
            // Pass an invalid bucket and prefix.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                SharePrefix::new("a\0bucket", "a/b\0/c")
                    .expect_err("new passing a bucket and prefix with NULL bytes")
            {
                assert_eq!(names, "bucket", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte)",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
    }

    #[test]
    fn test_permission_default() {
        let perm = Permission::new();

        assert!(!perm.allow_download, "allow download");
        assert!(!perm.allow_upload, "allow upload");
        assert!(!perm.allow_list, "allow list");
        assert!(!perm.allow_delete, "allow delete");
        assert_eq!(perm.not_before(), None, "not before");
        assert_eq!(perm.not_after(), None, "not after");
    }

    #[test]
    fn test_permission_full() {
        let perm = Permission::full();

        assert!(perm.allow_download, "allow download");
        assert!(perm.allow_upload, "allow upload");
        assert!(perm.allow_list, "allow list");
        assert!(perm.allow_delete, "allow delete");
        assert_eq!(perm.not_before(), None, "not before");
        assert_eq!(perm.not_after(), None, "not after");
    }

    #[test]
    fn test_permission_read_only() {
        let perm = Permission::read_only();

        assert!(perm.allow_download, "allow download");
        assert!(!perm.allow_upload, "allow upload");
        assert!(perm.allow_list, "allow list");
        assert!(!perm.allow_delete, "allow delete");
        assert_eq!(perm.not_before(), None, "not before");
        assert_eq!(perm.not_after(), None, "not after");
    }

    #[test]
    fn test_permission_write_only() {
        let perm = Permission::write_only();

        assert!(!perm.allow_download, "allow download");
        assert!(perm.allow_upload, "allow upload");
        assert!(!perm.allow_list, "allow list");
        assert!(perm.allow_delete, "allow delete");
        assert_eq!(perm.not_before(), None, "not before");
        assert_eq!(perm.not_after(), None, "not after");
    }

    #[test]
    fn test_permission_time_boundaries() {
        let mut perm = Permission::full();

        assert_eq!(perm.not_before(), None, "not before");
        assert_eq!(perm.not_after(), None, "not after");

        // set not before and after without violating their constraints.
        {
            perm.set_not_before(Some(Duration::new(5, 50)))
                .expect("set not before");
            assert_eq!(
                perm.not_before(),
                Some(Duration::new(5, 50)),
                "set not before"
            );

            perm.set_not_after(Some(Duration::new(5, 51)))
                .expect("set not after");
            assert_eq!(
                perm.not_after(),
                Some(Duration::new(5, 51)),
                "set not after"
            );
        }

        // set not before violating its constraints.
        {
            if let Error::InvalidArguments(error::Args { names, msg }) = perm
                .set_not_before(Some(Duration::new(5, 52)))
                .expect_err("set not before")
            {
                assert_eq!(names, "since", "invalid error argument name");
                assert_eq!(
                    msg,
                    "cannot be more recent or equal to the not after valid time of the permission",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }

        // set not after violating its constraints.
        {
            if let Error::InvalidArguments(error::Args { names, msg }) = perm
                .set_not_after(Some(Duration::new(5, 50)))
                .expect_err("set not after")
            {
                assert_eq!(names, "until", "invalid error argument name");
                assert_eq!(
                    msg,
                    "cannot be previous or equal to the not before valid time of the permission",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }

        // removing not before and after
        {
            perm.set_not_before(None).expect("set not before");
            assert_eq!(perm.not_before(), None, "removing not before");

            perm.set_not_after(None).expect("set not after");
            assert_eq!(perm.not_after(), None, "removing not after");
        }
    }
}
