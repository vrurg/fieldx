mod fxlock;
mod fxproxy;

pub use fxlock::FXRwLock;
#[doc(hidden)]
pub use fxproxy::FXBuilderFallible;
#[doc(hidden)]
pub use fxproxy::FXBuilderInfallible;
pub use fxproxy::FXProxy;
pub use fxproxy::FXProxyReadGuard;
pub use fxproxy::FXProxyWriteGuard;
pub use fxproxy::FXWriter;
pub use tokio::sync::OnceCell;
#[doc(hidden)]
pub use tokio::sync::RwLock;
#[doc(hidden)]
pub use tokio::sync::RwLockMappedWriteGuard;
#[doc(hidden)]
pub use tokio::sync::RwLockReadGuard;
#[doc(hidden)]
pub use tokio::sync::RwLockWriteGuard;

#[inline(always)]
pub fn new_lazy_container<T>(value: Option<T>) -> OnceCell<T> {
    if let Some(v) = value {
        OnceCell::from(v)
    }
    else {
        OnceCell::new()
    }
}
