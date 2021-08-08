//! Storj DCS Uplink idiomatic and safe Rust bindings.

#![deny(missing_docs)]

pub(crate) mod access;
pub(crate) mod encryption_key;
pub mod error;
pub(crate) mod helpers;
pub(crate) mod project;

pub use access::Access;
pub use access::Permission;
pub use access::SharePrefix;
pub use encryption_key::EncryptionKey;
pub use error::Error;
pub use project::Project;

/// An interface for ensuring that an instance of type returned by the
/// underlying c-binding is correct in terms that it doesn't violate its own
/// rules.
/// For example a UplinkAccessResult struct has 2 fields which are 2 pointers,
/// one is the access and the other is an error, always one and only one can be
/// NULL.
trait Ensurer {
    /// Checks that the instance is correct according its own rules and it
    /// returns itself, otherwise it panics.
    fn ensure(&self) -> &Self;
}
