use std::{borrow::Borrow, fmt, fmt::Debug, ops::Deref};
pub use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Default)]
pub struct FXRwLockAsync<T>(RwLock<T>);

impl<T> FXRwLockAsync<T> {
    #[doc(hidden)]
    pub fn new(value: T) -> Self {
        Self(RwLock::new(value))
    }

    /// Consumes the lock and returns the wrapped value.
    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }

    pub async fn read(&self) -> RwLockReadGuard<T> {
        self.0.read().await
    }

    pub async fn write(&self) -> RwLockWriteGuard<T> {
        self.0.write().await
    }
}

impl<T> PartialEq for FXRwLockAsync<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        let myguard = self.0.blocking_read();
        let otherguard = other.0.blocking_read();

        myguard.eq(&otherguard)
    }
}

impl<T> From<T> for FXRwLockAsync<T> {
    fn from(value: T) -> Self {
        Self(RwLock::new(value))
    }
}

impl<T> Deref for FXRwLockAsync<T> {
    type Target = RwLock<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> AsRef<RwLock<T>> for FXRwLockAsync<T> {
    fn as_ref(&self) -> &RwLock<T> {
        &self.0
    }
}

impl<T> Borrow<RwLock<T>> for FXRwLockAsync<T> {
    fn borrow(&self) -> &RwLock<T> {
        &self.0
    }
}

impl<T> Clone for FXRwLockAsync<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let vguard = self.0.blocking_read();
        Self(RwLock::new((*vguard).clone()))
    }
}

impl<T> Debug for FXRwLockAsync<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
