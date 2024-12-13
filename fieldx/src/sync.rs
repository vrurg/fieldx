pub mod fxlock;
pub mod fxproxy;

pub use fxlock::FXRwLockSync;
#[doc(hidden)]
pub use fxproxy::{FXBuilderFallible, FXBuilderInfallible};
pub use fxproxy::{FXProxySync, FXWrLockGuardSync};
#[doc(hidden)]
pub use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLockReadGuard, RwLockWriteGuard};
