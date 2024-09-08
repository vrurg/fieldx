use crate::traits::{FXNewDefault, FXStruct};
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard,
};
use std::{
    any,
    cell::RefCell,
    fmt,
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// Container type for lazy fields
///
/// Direct use of this struct is not recommended. See [reader and writer helpers](mod@crate#reader_writer_helpers).
pub struct FXProxy<S, T> {
    value:   RwLock<Option<T>>,
    is_set:  AtomicBool,
    builder: RwLock<Option<fn(&S) -> T>>,
}

/// Write-lock returned by [`FXProxy::write`] method
///
/// This type, in cooperation with the [`FXProxy`] type, takes care of safely updating lazy field status when data is
/// being stored.
pub struct FXWrLock<'a, S, T> {
    lock:     RefCell<RwLockWriteGuard<'a, Option<T>>>,
    fxproxy:  &'a FXProxy<S, T>,
    _phantom: PhantomData<S>,
}

impl<S, T: fmt::Debug> fmt::Debug for FXProxy<S, T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vlock = self.value.read();
        formatter
            .debug_struct(any::type_name::<Self>())
            .field("value", &*vlock)
            .finish()
    }
}

impl<S, T> FXNewDefault<S, T> for FXProxy<S, T>
where
    S: FXStruct,
{
    #[doc(hidden)]
    fn new_default(builder_method: fn(&S) -> T, value: Option<T>) -> Self {
        Self {
            is_set:  AtomicBool::new(value.is_some()),
            value:   RwLock::new(value),
            builder: RwLock::new(Some(builder_method)),
        }
    }
}

impl<S, T> FXNewDefault<Arc<S>, T> for FXProxy<Arc<S>, T>
where
    S: FXStruct,
{
    #[doc(hidden)]
    fn new_default(builder_method: fn(&Arc<S>) -> T, value: Option<T>) -> Self {
        Self {
            is_set:  AtomicBool::new(value.is_some()),
            value:   RwLock::new(value),
            builder: RwLock::new(Some(builder_method)),
        }
    }
}

impl<S, T> FXProxy<S, T> {
    /// Consumes the container, returns the wrapped value or None if the container is empty
    pub fn into_inner(self) -> Option<T> {
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
    pub fn lazy_init<'a>(&'a self, owner: &S) {
        let _ = self.read_or_init(owner);
    }

    fn read_or_init<'a>(&'a self, owner: &S) -> RwLockUpgradableReadGuard<'a, Option<T>> {
        let guard = self.value.upgradable_read();
        if (*guard).is_none() {
            let mut wguard = RwLockUpgradableReadGuard::upgrade(guard);
            // Still uninitialized? Means no other thread took care of it yet.
            if wguard.is_none() {
                // No value has been set yet
                match *self.builder.read() {
                    Some(ref builder_cb) => {
                        *wguard = Some((*builder_cb)(owner));
                        self.is_set_raw().store(true, Ordering::SeqCst);
                    }
                    None => panic!("Builder is not set"),
                }
            }
            RwLockWriteGuard::downgrade_to_upgradable(wguard)
        }
        else {
            guard
        }
    }

    /// Since the container guarantees that reading from it initializes the wrapped value, this method provides
    /// semit-direct access to it without the [`Option`] wrapper.
    pub fn read<'a>(&'a self, owner: &S) -> MappedRwLockReadGuard<'a, T> {
        RwLockReadGuard::map(
            RwLockUpgradableReadGuard::downgrade(self.read_or_init(owner)),
            |data: &Option<T>| data.as_ref().unwrap(),
        )
    }

    /// Since the container guarantees that reading from it initializes the wrapped value, this method provides
    /// semit-direct mutable access to it without the [`Option`] wrapper.
    pub fn read_mut<'a>(&'a self, owner: &S) -> MappedRwLockWriteGuard<'a, T> {
        RwLockWriteGuard::map(
            RwLockUpgradableReadGuard::upgrade(self.read_or_init(owner)),
            |data: &mut Option<T>| data.as_mut().unwrap(),
        )
    }

    /// Provides write-lock to directly store the value.
    pub fn write<'a>(&'a self) -> FXWrLock<'a, S, T> {
        FXWrLock::<'a, S, T>::new(self.value.write(), self)
    }

    fn clear_with_lock(&self, wguard: &mut RwLockWriteGuard<Option<T>>) -> Option<T> {
        self.is_set_raw().store(false, Ordering::SeqCst);
        wguard.take()
    }

    /// Resets the container into unitialized state
    pub fn clear(&self) -> Option<T> {
        let mut wguard = self.value.write();
        self.clear_with_lock(&mut wguard)
    }
}

#[allow(private_bounds)]
impl<'a, S, T> FXWrLock<'a, S, T> {
    #[doc(hidden)]
    pub fn new(lock: RwLockWriteGuard<'a, Option<T>>, fxproxy: &'a FXProxy<S, T>) -> Self {
        let lock = RefCell::new(lock);
        Self {
            lock,
            fxproxy,
            _phantom: PhantomData,
        }
    }

    /// Store a new value into the container and returns the previous value or `None`.
    pub fn store(&mut self, value: T) -> Option<T> {
        self.fxproxy.is_set_raw().store(true, Ordering::Release);
        self.lock.borrow_mut().replace(value)
    }

    /// Resets the container into unitialized state
    pub fn clear(&self) -> Option<T> {
        self.fxproxy.clear_with_lock(&mut *self.lock.borrow_mut())
    }
}

impl<S, T> Clone for FXProxy<S, T>
where
    S: FXStruct,
    T: Clone,
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

impl<S, T> PartialEq for FXProxy<S, T>
where
    S: FXStruct,
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        let myguard = self.value.read();
        let otherguard = other.value.read();
        myguard.eq(&otherguard)
    }
}

impl<S, T> Eq for FXProxy<S, T>
where
    S: FXStruct,
    T: Eq,
{
}
