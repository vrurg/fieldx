use parking_lot::RwLock;
use std::{borrow::Borrow, fmt, fmt::Debug, ops::Deref};

/// Lock-protected container
///
/// This is a wrapper around [`RwLock`] sync primitive. It provides safe means of cloning the lock and the data it
/// protects.
#[derive(Default)]
pub struct FXRwLockSync<T>(RwLock<T>);

impl<T> FXRwLockSync<T> {
    #[doc(hidden)]
    pub fn new(value: T) -> Self {
        Self(RwLock::new(value))
    }

    /// Consumes the lock and returns the wrapped value.
    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }
}

impl<T> PartialEq for FXRwLockSync<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        let myguard = self.0.read();
        let otherguard = other.0.read();

        myguard.eq(&otherguard)
    }
}

impl<T> Eq for FXRwLockSync<T> where T: Eq {}

impl<T> From<T> for FXRwLockSync<T> {
    fn from(value: T) -> Self {
        Self(RwLock::new(value))
    }
}

impl<T> Deref for FXRwLockSync<T> {
    type Target = RwLock<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> AsRef<RwLock<T>> for FXRwLockSync<T> {
    fn as_ref(&self) -> &RwLock<T> {
        &self.0
    }
}

impl<T> Borrow<RwLock<T>> for FXRwLockSync<T> {
    fn borrow(&self) -> &RwLock<T> {
        &self.0
    }
}

impl<T> Clone for FXRwLockSync<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let vguard = self.0.read();
        Self(RwLock::new((*vguard).clone()))
    }
}

impl<T> Debug for FXRwLockSync<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
