//! Storj DCS Encryption key

use uplink_sys as ulksys;

/// TODO: implement & document it
pub struct EncryptionKey {
    inner: ulksys::UplinkEncryptionKeyResult,
}

impl EncryptionKey {
    pub(crate) fn into_raw_mut(&self) -> *mut ulksys::UplinkEncryptionKey {
        self.inner.encryption_key
    }
}
