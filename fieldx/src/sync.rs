pub mod fxlock;
pub mod fxproxy;

pub use fxlock::FXRwLock;
pub use fxproxy::{FXProxy, FXWrLockGuard};
#[doc(hidden)]
pub use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard,
};
