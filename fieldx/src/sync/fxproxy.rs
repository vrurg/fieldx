use crate::traits::{FXBuilderWrapper, FXStruct};
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard,
};
use std::{
    any,
    cell::RefCell,
    fmt::{self, Debug, Formatter},
    sync::atomic::{AtomicBool, Ordering},
};

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
    // builder: RwLock<Option<Box<dyn FXBuilderWrapper<Owner = S, Value = T, Error = E>>>>,
    builder: RwLock<Option<B>>,
}

/// Write-lock returned by [`FXProxy::write`] method
///
/// This type, in cooperation with the [`FXProxy`] type, takes care of safely updating lazy field status when data is
/// being stored.
pub struct FXWrLockGuard<'a, B>
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

    /// Initialize the field without obtaining the lock. Note though that if the lock is already owned this method will
    /// wait for it to be released.
    pub fn lazy_init<'a>(&'a self, owner: &B::Owner) {
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

    /// Since the container guarantees that reading from it initializes the wrapped value, this method provides
    /// semit-direct access to it without the [`Option`] wrapper.
    pub fn read<'a>(&'a self, owner: &B::Owner) -> MappedRwLockReadGuard<'a, B::Value> {
        RwLockReadGuard::map(
            RwLockUpgradableReadGuard::downgrade(self.read_or_init(owner).unwrap()),
            |data: &Option<B::Value>| data.as_ref().unwrap(),
        )
    }

    /// Since the container guarantees that reading from it initializes the wrapped value, this method provides
    /// semit-direct mutable access to it without the [`Option`] wrapper.
    pub fn read_mut<'a>(&'a self, owner: &B::Owner) -> MappedRwLockWriteGuard<'a, B::Value> {
        RwLockWriteGuard::map(
            RwLockUpgradableReadGuard::upgrade(self.read_or_init(owner).unwrap()),
            |data: &mut Option<B::Value>| data.as_mut().unwrap(),
        )
    }

    /// Since the container guarantees that reading from it initializes the wrapped value, this method provides
    /// semit-direct access to it without the [`Option`] wrapper.
    pub fn try_read<'a>(&'a self, owner: &B::Owner) -> Result<MappedRwLockReadGuard<'a, B::Value>, B::Error> {
        Ok(RwLockReadGuard::map(
            RwLockUpgradableReadGuard::downgrade(self.read_or_init(owner)?),
            |data: &Option<B::Value>| data.as_ref().unwrap(),
        ))
    }

    /// Since the container guarantees that reading from it initializes the wrapped value, this method provides
    /// semit-direct mutable access to it without the [`Option`] wrapper.
    pub fn try_read_mut<'a>(&'a self, owner: &B::Owner) -> Result<MappedRwLockWriteGuard<'a, B::Value>, B::Error> {
        Ok(RwLockWriteGuard::map(
            RwLockUpgradableReadGuard::upgrade(self.read_or_init(owner)?),
            |data: &mut Option<B::Value>| data.as_mut().unwrap(),
        ))
    }

    /// Provides write-lock to directly store the value.
    pub fn write<'a>(&'a self) -> FXWrLockGuard<'a, B> {
        FXWrLockGuard::<'a, B>::new(self.value.write(), self)
    }

    fn clear_with_lock(&self, wguard: &mut RwLockWriteGuard<Option<B::Value>>) -> Option<B::Value> {
        self.is_set_raw().store(false, Ordering::SeqCst);
        wguard.take()
    }

    /// Resets the container into unitialized state
    pub fn clear(&self) -> Option<B::Value> {
        let mut wguard = self.value.write();
        self.clear_with_lock(&mut wguard)
    }
}

#[allow(private_bounds)]
impl<'a, B> FXWrLockGuard<'a, B>
where
    B: FXBuilderWrapperSync,
{
    #[doc(hidden)]
    pub fn new(lock: RwLockWriteGuard<'a, Option<B::Value>>, fxproxy: &'a FXProxy<B>) -> Self {
        let lock = RefCell::new(lock);
        Self { lock, fxproxy }
    }

    /// Store a new value into the container and returns the previous value or `None`.
    pub fn store(&mut self, value: B::Value) -> Option<B::Value> {
        self.fxproxy.is_set_raw().store(true, Ordering::Release);
        self.lock.borrow_mut().replace(value)
    }

    /// Resets the container into unitialized state
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
