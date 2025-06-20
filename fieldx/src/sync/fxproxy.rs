use crate::traits::FXBuilderWrapper;
use crate::traits::FXStruct;
use parking_lot::RwLock;
use parking_lot::RwLockReadGuard;
use parking_lot::RwLockUpgradableReadGuard;
use parking_lot::RwLockWriteGuard;
use std::any;
use std::cell::RefCell;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::{self};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

pub type FXProxyReadGuard<'a, T> = crate::lock_guards::FXProxyReadGuard<RwLockReadGuard<'a, Option<T>>, T>;
pub type FXProxyWriteGuard<'a, T> = crate::lock_guards::FXProxyWriteGuard<RwLockWriteGuard<'a, Option<T>>, T>;

#[doc(hidden)]
pub trait FXBuilderWrapperSync: FXBuilderWrapper {
    fn invoke(&self, owner: &Self::Owner) -> Result<Self::Value, Self::Error>;
}

#[doc(hidden)]
#[derive(Clone)]
pub struct FXBuilderInfallible<S, T> {
    builder: fn(&S) -> T,
}

impl<S, T> FXBuilderInfallible<S, T> {
    pub fn new(builder: fn(&S) -> T) -> Self {
        Self { builder }
    }
}

impl<S: FXStruct, T> FXBuilderWrapper for FXBuilderInfallible<S, T> {
    type Error = ();
    type Owner = S;
    type Value = T;
}

impl<S: FXStruct, T> FXBuilderWrapperSync for FXBuilderInfallible<S, T> {
    #[inline(always)]
    fn invoke(&self, owner: &Self::Owner) -> Result<Self::Value, Self::Error> {
        Ok((self.builder)(owner))
    }
}

#[doc(hidden)]
#[derive(Clone)]
pub struct FXBuilderFallible<S, T, E> {
    builder: fn(&S) -> Result<T, E>,
}

impl<S, T, E> FXBuilderFallible<S, T, E> {
    pub fn new(builder: fn(&S) -> Result<T, E>) -> Self {
        Self { builder }
    }
}

impl<S: FXStruct, T, E: Debug> FXBuilderWrapper for FXBuilderFallible<S, T, E> {
    type Error = E;
    type Owner = S;
    type Value = T;
}

impl<S: FXStruct, T, E: Debug> FXBuilderWrapperSync for FXBuilderFallible<S, T, E> {
    #[inline(always)]
    fn invoke(&self, owner: &Self::Owner) -> Result<Self::Value, Self::Error> {
        (self.builder)(owner)
    }
}

/// Container type for lazy fields
pub struct FXProxy<B>
where
    B: FXBuilderWrapperSync,
{
    value:   RwLock<Option<B::Value>>,
    is_set:  AtomicBool,
    builder: RwLock<Option<B>>,
}

/// Write-lock returned by [`FXProxy::write`] method
///
/// This type, in cooperation with the [`FXProxy`] type, takes care of safely updating lazy field status when data
/// is being stored.
pub struct FXWriter<'a, B>
where
    B: FXBuilderWrapperSync,
{
    lock:    RefCell<RwLockWriteGuard<'a, Option<B::Value>>>,
    fxproxy: &'a FXProxy<B>,
}

impl<B, V> Debug for FXProxy<B>
where
    B: FXBuilderWrapperSync<Value = V>,
    V: Debug,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let vlock = self.value.read();
        formatter
            .debug_struct(any::type_name::<Self>())
            .field("value", &*vlock)
            .finish()
    }
}

impl<B> FXProxy<B>
where
    B: FXBuilderWrapperSync,
{
    #[doc(hidden)]
    pub fn new_default(builder: B, value: Option<B::Value>) -> Self {
        Self {
            is_set:  AtomicBool::new(value.is_some()),
            value:   RwLock::new(value),
            builder: RwLock::new(Some(builder)),
        }
    }

    /// Consumes the container, returns the wrapped value or None if the container is empty
    pub fn into_inner(self) -> Option<B::Value> {
        self.value.into_inner()
    }

    #[inline]
    fn is_set_raw(&self) -> &AtomicBool {
        &self.is_set
    }

    /// Returns `true` if the container has a value.
    #[inline]
    pub fn is_set(&self) -> bool {
        self.is_set_raw().load(Ordering::SeqCst)
    }

    /// Initialize the field without obtaining the lock by calling code. _Note_ though that internally the lock is still
    /// required.
    pub fn lazy_init(&self, owner: &B::Owner) {
        let _ = self.read_or_init(owner);
    }

    fn read_or_init<'a>(
        &'a self,
        owner: &B::Owner,
    ) -> Result<RwLockUpgradableReadGuard<'a, Option<B::Value>>, B::Error> {
        let guard = self.value.upgradable_read();
        Ok(if (*guard).is_none() {
            let mut wguard = RwLockUpgradableReadGuard::upgrade(guard);
            // Still uninitialized? Means no other thread took care of it yet.
            if wguard.is_none() {
                // No value has been set yet
                match *self.builder.read() {
                    Some(ref builder_cb) => {
                        *wguard = Some((*builder_cb).invoke(owner)?);
                        self.is_set_raw().store(true, Ordering::SeqCst);
                    }
                    None => panic!("Builder is not set"),
                }
            }
            RwLockWriteGuard::downgrade_to_upgradable(wguard)
        }
        else {
            guard
        })
    }

    /// Lazy-initialize the field if necessary and return lock read guard for the inner value.
    ///
    /// Panics if fallible field builder returns an error.
    pub fn read<'a>(&'a self, owner: &B::Owner) -> FXProxyReadGuard<'a, B::Value> {
        FXProxyReadGuard::new(RwLockUpgradableReadGuard::downgrade(self.read_or_init(owner).unwrap()))
    }

    /// Lazy-initialize the field if necessary and return lock write guard for the inner value.
    ///
    /// Panics if fallible field builder returns an error.
    pub fn read_mut<'a>(&'a self, owner: &B::Owner) -> FXProxyWriteGuard<'a, B::Value> {
        FXProxyWriteGuard::new(RwLockUpgradableReadGuard::upgrade(self.read_or_init(owner).unwrap()))
    }

    /// Lazy-initialize the field if necessary and return lock read guard for the inner value.
    ///
    /// Return the same error, as fallible field builder if it errors out.
    pub fn try_read<'a>(&'a self, owner: &B::Owner) -> Result<FXProxyReadGuard<'a, B::Value>, B::Error> {
        Ok(FXProxyReadGuard::new(RwLockUpgradableReadGuard::downgrade(
            self.read_or_init(owner)?,
        )))
    }

    /// Lazy-initialize the field if necessary and return lock write guard for the inner value.
    ///
    /// Return the same error, as fallible field builder if it errors out.
    pub fn try_read_mut<'a>(&'a self, owner: &B::Owner) -> Result<FXProxyWriteGuard<'a, B::Value>, B::Error> {
        Ok(FXProxyWriteGuard::new(RwLockUpgradableReadGuard::upgrade(
            self.read_or_init(owner)?,
        )))
    }

    /// Provides write-lock to directly store the value. Never calls the lazy builder.
    pub fn write<'a>(&'a self) -> FXWriter<'a, B> {
        FXWriter::<'a, B>::new(self.value.write(), self)
    }

    fn clear_with_lock(&self, wguard: &mut RwLockWriteGuard<Option<B::Value>>) -> Option<B::Value> {
        self.is_set_raw().store(false, Ordering::SeqCst);
        wguard.take()
    }

    /// Resets the container into uninitialized state
    pub fn clear(&self) -> Option<B::Value> {
        let mut wguard = self.value.write();
        self.clear_with_lock(&mut wguard)
    }
}

#[allow(private_bounds)]
impl<'a, B> FXWriter<'a, B>
where
    B: FXBuilderWrapperSync,
{
    #[doc(hidden)]
    pub(crate) fn new(lock: RwLockWriteGuard<'a, Option<B::Value>>, fxproxy: &'a FXProxy<B>) -> Self {
        let lock = RefCell::new(lock);
        Self { lock, fxproxy }
    }

    /// Store a new value into the container and returns the previous value or `None`.
    pub fn store(&mut self, value: B::Value) -> Option<B::Value> {
        self.fxproxy.is_set_raw().store(true, Ordering::Release);
        self.lock.borrow_mut().replace(value)
    }

    /// Resets the container into uninitialized state
    pub fn clear(&self) -> Option<B::Value> {
        self.fxproxy.clear_with_lock(&mut *self.lock.borrow_mut())
    }
}

impl<B, V> Clone for FXProxy<B>
where
    B: FXBuilderWrapperSync<Value = V> + Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        let vguard = self.value.read();
        let bguard = self.builder.read();
        Self {
            value:   RwLock::new((*vguard).as_ref().cloned()),
            is_set:  AtomicBool::new(self.is_set()),
            builder: RwLock::new(bguard.clone()),
        }
    }
}

impl<B, V> PartialEq for FXProxy<B>
where
    B: FXBuilderWrapperSync<Value = V>,
    V: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        let myguard = self.value.read();
        let otherguard = other.value.read();
        myguard.eq(&otherguard)
    }
}

impl<B, V> Eq for FXProxy<B>
where
    B: FXBuilderWrapperSync<Value = V>,
    V: Eq,
{
}
