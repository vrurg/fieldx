pub mod fxlock;
pub mod fxproxy;

pub use fxlock::FXRwLockSync;
pub use fxproxy::{FXBuilderFallible, FXBuilderInfallible, FXProxySync, FXWrLockGuardSync};
#[doc(hidden)]
pub use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard,
};
