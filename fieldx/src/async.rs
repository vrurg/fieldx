mod fxlock;
mod fxproxy;

pub use fxlock::FXRwLockAsync;
#[doc(hidden)]
pub use fxproxy::FXBuilderFallible;
#[doc(hidden)]
pub use fxproxy::FXBuilderInfallible;
pub use fxproxy::FXProxyAsync;
pub use fxproxy::FXWrLockGuardAsync;
#[doc(hidden)]
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
