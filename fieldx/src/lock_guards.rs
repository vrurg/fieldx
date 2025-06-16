use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::fmt;
use std::ops::Deref;
use std::ops::DerefMut;

pub struct FXProxyReadGuard<G, T>
where
    G: Deref<Target = Option<T>>,
{
    guard: G,
}

impl<G, T> FXProxyReadGuard<G, T>
where
    G: Deref<Target = Option<T>>,
{
    pub fn new(guard: G) -> Self {
        Self { guard }
    }
}

impl<G, T> Deref for FXProxyReadGuard<G, T>
where
    G: Deref<Target = Option<T>>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.deref().as_ref().unwrap()
    }
}

impl<G, T> AsRef<T> for FXProxyReadGuard<G, T>
where
    G: Deref<Target = Option<T>>,
{
    fn as_ref(&self) -> &T {
        self.guard.deref().as_ref().unwrap()
    }
}

impl<G, T> Borrow<T> for FXProxyReadGuard<G, T>
where
    G: Deref<Target = Option<T>>,
{
    fn borrow(&self) -> &T {
        self.guard.deref().as_ref().unwrap()
    }
}

impl<G, T> fmt::Debug for FXProxyReadGuard<G, T>
where
    G: Deref<Target = Option<T>>,
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.guard.deref().as_ref().unwrap().fmt(f)
    }
}

pub struct FXProxyWriteGuard<G, T>
where
    G: DerefMut<Target = Option<T>> + Deref<Target = Option<T>>,
{
    guard: G,
}

impl<G, T> FXProxyWriteGuard<G, T>
where
    G: DerefMut<Target = Option<T>> + Deref<Target = Option<T>>,
{
    pub fn new(guard: G) -> Self {
        Self { guard }
    }
}

impl<G, T> Deref for FXProxyWriteGuard<G, T>
where
    G: DerefMut<Target = Option<T>> + Deref<Target = Option<T>>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.deref().as_ref().unwrap()
    }
}

impl<G, T> AsRef<T> for FXProxyWriteGuard<G, T>
where
    G: DerefMut<Target = Option<T>> + Deref<Target = Option<T>>,
{
    fn as_ref(&self) -> &T {
        self.guard.deref().as_ref().unwrap()
    }
}

impl<G, T> Borrow<T> for FXProxyWriteGuard<G, T>
where
    G: DerefMut<Target = Option<T>> + Deref<Target = Option<T>>,
{
    fn borrow(&self) -> &T {
        self.guard.deref().as_ref().unwrap()
    }
}

impl<G, T> DerefMut for FXProxyWriteGuard<G, T>
where
    G: DerefMut<Target = Option<T>> + Deref<Target = Option<T>>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut().as_mut().unwrap()
    }
}

impl<G, T> AsMut<T> for FXProxyWriteGuard<G, T>
where
    G: DerefMut<Target = Option<T>> + Deref<Target = Option<T>>,
{
    fn as_mut(&mut self) -> &mut T {
        self.guard.deref_mut().as_mut().unwrap()
    }
}

impl<G, T> BorrowMut<T> for FXProxyWriteGuard<G, T>
where
    G: DerefMut<Target = Option<T>> + Deref<Target = Option<T>>,
{
    fn borrow_mut(&mut self) -> &mut T {
        self.guard.deref_mut().as_mut().unwrap()
    }
}

impl<G, T> fmt::Debug for FXProxyWriteGuard<G, T>
where
    G: DerefMut<Target = Option<T>> + Deref<Target = Option<T>>,
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.guard.deref().as_ref().unwrap().fmt(f)
    }
}
