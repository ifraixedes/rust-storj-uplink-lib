//! Storj DCS Uplink configuration.

use crate::{helpers, Result};

use std::time::Duration;

use uplink_sys as ulksys;

/// Defines configuration for using Uplink library.
#[derive(Debug)]
pub struct Config<'a> {
    /// The configuration type of the underlying c-bindings Rust crate that an
    /// instance of this struct represents and guard its life time until this
    /// instance drops.
    pub(crate) inner: ulksys::UplinkConfig,

    /// Identifies the application how is contacting with the satellite.
    /// The user agent is used for statistics and for identifying the usage
    /// coming from associated partners.
    user_agent: &'a str,
    /// Defines how long the client should wait for establishing a connection to
    /// peers.
    dial_timeout: Duration,
    /// Path to a directory to be used for storing temporary files when running
    /// completely in memory is disabled. It's `None` when running only in
    /// memory.
    temp_dir: Option<&'a str>,
    /// Specifies to only operates using memory, hence it doesn't off-load data
    /// to disk.
    in_memory: bool,
}

impl<'a> Config<'a> {
    /// Creates a configuration with the specific user agent and dial timeout.
    /// All the operations performed by this configuration or any instance
    /// created from it will operate entirely in memory.
    fn new(user_agent: &'a str, dial_timeout: Duration) -> Result<Self> {
        let inner;
        {
            use std::ptr::null;

            let uagent = helpers::cstring_from_str_fn_arg("user_agent", user_agent)?;
            inner = ulksys::UplinkConfig {
                user_agent: uagent.into_raw(),
                dial_timeout_milliseconds: dial_timeout.as_millis() as i32,
                temp_directory: null(),
            };
        }

        Ok(Config {
            inner,
            user_agent,
            dial_timeout,
            temp_dir: None,
            in_memory: true,
        })
    }

    /// Creates a configuration with the specific user agent, dial timeout and
    /// using a specific directory path for creating temporary files.
    /// Some operations performed by this configuration or any instance
    /// created from it may offload data from memory to disk.
    /// When `temp_dir`is `None`, a random directory path will be used.
    ///
    /// NOTE even that the underlying c-binding offers this option, it may not
    /// use it and just fully operates in memory.
    fn new_use_disk(
        user_agent: &'a str,
        dial_timeout: Duration,
        temp_dir: Option<&'a str>,
    ) -> Result<Self> {
        let inner;
        {
            let uagent = helpers::cstring_from_str_fn_arg("user_agent", user_agent)?;
            let tdir = temp_dir.or(Some("")).unwrap();
            let tdir = helpers::cstring_from_str_fn_arg("temp_dir", tdir)?;

            inner = ulksys::UplinkConfig {
                user_agent: uagent.into_raw(),
                dial_timeout_milliseconds: dial_timeout.as_millis() as i32,
                temp_directory: tdir.into_raw(),
            };
        }

        Ok(Config {
            inner,
            user_agent,
            dial_timeout,
            temp_dir,
            in_memory: false,
        })
    }
}

impl<'a> Drop for Config<'a> {
    fn drop(&mut self) {
        use std::ffi::CString;
        use std::os::raw::c_char;

        // SAFETY: The inner field is initialized when an instance of this
        // struct is initialized and it's only used by this crate to passed
        // to the underlying c-bindings.
        // The underlying c-bindings never free the memory or mutate the fields
        // of its exposed struct instance held by the inner field, hence the
        // life time of its fields which are pointers belong to this instance,
        // so they are freed when this instance drops.
        // The 2 pointers explicitly freed here came from the call to the
        // `into_raw` method of the `CString` instances crated from `&str`.
        // Because this method transfers the ownership to the returned raw
        // pointer, Rust doesn't know about their lifetime and we have to free
        // the memory manually.
        unsafe {
            // `self.inner.user_agent` is never null, otherwise there is bug in
            // the implementation of this struct.
            drop(CString::from_raw(self.inner.user_agent as *mut c_char));

            if !self.inner.temp_directory.is_null() {
                drop(CString::from_raw(self.inner.temp_directory as *mut c_char));
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::helpers::test::assert_c_string;
    use crate::{error, Error};

    #[test]
    fn test_new() {
        {
            // OK case.
            let config = Config::new("rust-bindings", Duration::new(3, 0))
                .expect("new shouldn't fail when 'user agent' doesn't contain any null character");

            assert_eq!(config.user_agent, "rust-bindings", "user_agent");
            assert_eq!(config.dial_timeout, Duration::new(3, 0), "dial_timeout");
            assert_eq!(config.temp_dir, None, "temp_dir");
            assert!(config.in_memory, "in_memory");

            assert_c_string(config.inner.user_agent, "rust-bindings");
            assert_eq!(
                config.inner.temp_directory,
                std::ptr::null(),
                "config.inner.temp_directory should be null"
            );
            assert_eq!(
                config.inner.dial_timeout_milliseconds, 3000,
                "inner.dial_tiemout_milliseconds"
            );
        }
        {
            // Error case.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                Config::new("rust\0bindings", Duration::new(3, 0))
                    .expect_err("new passing a user agent with NULL bytes")
            {
                assert_eq!(names, "user_agent", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 4",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
    }

    #[test]
    fn test_new_use_disk() {
        {
            // OK case: use a randomly generated temp directory.
            let config =
                Config::new_use_disk("rust-bindings-use-disk", Duration::new(2, 5000000), None)
                    .expect(
                        "new shouldn't fail when 'user agent' doesn't contain any null character",
                    );

            assert_eq!(config.user_agent, "rust-bindings-use-disk", "user_agent");
            assert_eq!(
                config.dial_timeout,
                Duration::new(2, 5000000),
                "dial_timeout"
            );
            assert_eq!(config.temp_dir, None, "temp_dir");
            assert!(!config.in_memory, "in_memory");

            assert_c_string(config.inner.user_agent, "rust-bindings-use-disk");
            assert_ne!(config.inner.temp_directory, std::ptr::null());
            assert_eq!(
                config.inner.dial_timeout_milliseconds, 2005,
                "inner.dial_tiemout_milliseconds"
            );
        }
        {
            // OK case: use a specific temp directory.
            let config = Config::new_use_disk(
                "rust-bindings-specify-temp-dir",
                Duration::new(1, 785999999),
                Some("/tmp/rust-uplink"),
            )
            .expect("new shouldn't fail when 'user agent' doesn't contain any null character");

            assert_eq!(
                config.user_agent, "rust-bindings-specify-temp-dir",
                "user_agent"
            );
            assert_eq!(
                config.dial_timeout,
                Duration::new(1, 785999999),
                "dial_timeout"
            );
            assert_eq!(config.temp_dir, Some("/tmp/rust-uplink"), "temp_dir");
            assert!(!config.in_memory, "in_memory");

            assert_c_string(config.inner.user_agent, "rust-bindings-specify-temp-dir");
            assert_c_string(config.inner.temp_directory, "/tmp/rust-uplink");
            assert_eq!(
                config.inner.dial_timeout_milliseconds, 1785,
                "inner.dial_tiemout_milliseconds"
            );
        }
        {
            // Error case: User agent has null characters.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                Config::new_use_disk("rust-bindings\0", Duration::new(3, 0), None)
                    .expect_err("new passing a user agent with NULL bytes")
            {
                assert_eq!(names, "user_agent", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 13",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
        {
            // Error case: Temp directory has null characters.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                Config::new_use_disk("rust-bindings", Duration::new(3, 0), Some("\0invalid"))
                    .expect_err("new passing a user agent with NULL bytes")
            {
                assert_eq!(names, "temp_dir", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 0",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
    }
}
