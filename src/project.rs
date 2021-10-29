//! Storj DCS Project.

use uplink_sys as ulksys;

/// TODO: document it.
pub struct Project {}

impl Project {
    /// TODO: implement & document this method.
    pub(crate) fn from_uplink_c(uc_project: *mut ulksys::UplinkProject) -> Self {
        Self {}
    }

    /// TODO: implement & document this method.
    pub fn revoke_access(&self) {
        todo!("implement it")
    }
}
