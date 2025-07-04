use parking_lot::RwLock;
use std::borrow::Borrow;
use std::fmt;
use std::fmt::Debug;
use std::ops::Deref;

/// Lock-protected container
///
/// This is a wrapper around the [`RwLock`] synchronization primitive. It provides a safe way to clone a lock
/// and the data it protects, as well as compare two locked values. No additional functionality is provided.
pub struct FXRwLock<T>(RwLock<T>);

impl<T> FXRwLock<T> {
    #[doc(hidden)]
    pub fn new(value: T) -> Self {
        Self(RwLock::new(value))
    }

    /// Consumes the lock and returns the wrapped value.
    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }
}

impl<T> From<T> for FXRwLock<T> {
    fn from(value: T) -> Self {
        Self(RwLock::new(value))
    }
}

impl<T> Deref for FXRwLock<T> {
    type Target = RwLock<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> AsRef<RwLock<T>> for FXRwLock<T> {
    fn as_ref(&self) -> &RwLock<T> {
        &self.0
    }
}

impl<T> Borrow<RwLock<T>> for FXRwLock<T> {
    fn borrow(&self) -> &RwLock<T> {
        &self.0
    }
}

impl<T> Clone for FXRwLock<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let vguard = self.0.read();
        Self(RwLock::new((*vguard).clone()))
    }
}

impl<T> Debug for FXRwLock<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Default> Default for FXRwLock<T> {
    fn default() -> Self {
        Self(RwLock::new(T::default()))
    }
}
