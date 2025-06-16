pub mod fxlock;
pub mod fxproxy;

pub use fxlock::FXRwLock;
#[doc(hidden)]
pub use fxproxy::FXBuilderFallible;
#[doc(hidden)]
pub use fxproxy::FXBuilderInfallible;
pub use fxproxy::FXProxy;
pub use fxproxy::FXProxyReadGuard;
pub use fxproxy::FXProxyWriteGuard;
pub use fxproxy::FXWriter;
pub use once_cell::sync::OnceCell;
#[doc(hidden)]
pub use parking_lot::MappedRwLockReadGuard;
#[doc(hidden)]
pub use parking_lot::MappedRwLockWriteGuard;
#[doc(hidden)]
pub use parking_lot::RwLockReadGuard;
#[doc(hidden)]
pub use parking_lot::RwLockWriteGuard;

#[inline(always)]
pub fn new_lazy_container<T>(value: Option<T>) -> OnceCell<T> {
    if let Some(v) = value {
        OnceCell::from(v)
    }
    else {
        OnceCell::new()
    }
}
