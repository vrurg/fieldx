#[cfg(feature = "clonable-lock")]
mod fxlock;
mod fxproxy;

#[cfg(all(feature = "async-tokio", feature = "async-lock", not(docsrs)))]
compile_error!(
    "Both `async-tokio` and `async-lock` features cannot be enabled at the same time. Please, choose one of them."
);

#[cfg(not(any(feature = "async-tokio", feature = "async-lock")))]
compile_error!("Either `async-tokio` or `async-lock` feature must be enabled. Please, choose one of them.");

#[cfg(feature = "clonable-lock")]
pub use fxlock::FXRwLock;
#[doc(hidden)]
pub use fxproxy::FXBuilderFallible;
#[doc(hidden)]
pub use fxproxy::FXBuilderInfallible;
pub use fxproxy::FXProxy;
pub use fxproxy::FXProxyReadGuard;
pub use fxproxy::FXProxyWriteGuard;
pub use fxproxy::FXWriter;
#[cfg(feature = "async-tokio")]
#[cfg_attr(feature = "async-tokio", doc(hidden))]
pub use tokio::sync::OnceCell;
#[cfg(feature = "async-tokio")]
#[cfg_attr(feature = "async-tokio", doc(hidden))]
pub use tokio::sync::RwLock;
#[cfg(feature = "async-tokio")]
#[cfg_attr(feature = "async-tokio", doc(hidden))]
pub use tokio::sync::RwLockReadGuard;
#[cfg(feature = "async-tokio")]
#[cfg_attr(feature = "async-tokio", doc(hidden))]
pub use tokio::sync::RwLockWriteGuard;

#[cfg(all(feature = "async-lock", not(docsrs)))]
#[cfg_attr(feature = "async-lock", doc(hidden))]
pub use async_lock::OnceCell;
#[cfg(all(feature = "async-lock", not(docsrs)))]
#[cfg_attr(feature = "async-lock", doc(hidden))]
pub use async_lock::RwLock;
#[cfg(all(feature = "async-lock", not(docsrs)))]
#[cfg_attr(feature = "async-lock", doc(hidden))]
pub use async_lock::RwLockReadGuard;
#[cfg(all(feature = "async-lock", not(docsrs)))]
#[cfg_attr(feature = "async-lock", doc(hidden))]
pub use async_lock::RwLockWriteGuard;

#[inline(always)]
pub fn new_lazy_container<T>(value: Option<T>) -> OnceCell<T> {
    if let Some(v) = value {
        OnceCell::from(v)
    }
    else {
        OnceCell::new()
    }
}
