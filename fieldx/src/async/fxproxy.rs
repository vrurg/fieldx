use crate::traits::{FXNewDefault, FXStruct};
// use async_lock::{RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard};
// use parking_lot::{
//     MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard,
// };
use std::{
    any,
    cell::RefCell,
    fmt,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::sync::{RwLock, RwLockMappedWriteGuard, RwLockReadGuard, RwLockWriteGuard};

type FXCallback<S, T> = Box<dyn Fn(&S) -> Pin<Box<dyn Future<Output = T> + Send + '_>> + Send + Sync>;

/// Container type for lazy fields
pub struct FXProxyAsync<S, T> {
    value:   RwLock<Option<T>>,
    is_set:  AtomicBool,
    builder: RwLock<Option<FXCallback<S, T>>>,
}

/// Write-lock returned by [`FXProxyAsync::write`] method
///
/// This type, in cooperation with the [`FXProxyAsync`] type, takes care of safely updating lazy field status when data is
/// being stored.
pub struct FXWrLockGuardAsync<'a, S, T> {
    lock:     RefCell<RwLockWriteGuard<'a, Option<T>>>,
    fxproxy:  &'a FXProxyAsync<S, T>,
    _phantom: PhantomData<S>,
}

impl<S, T: fmt::Debug> fmt::Debug for FXProxyAsync<S, T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vlock = self.value.blocking_read();
        formatter
            .debug_struct(any::type_name::<Self>())
            .field("value", &*vlock)
            .finish()
    }
}

impl<S, T> FXNewDefault for FXProxyAsync<S, T>
where
    S: FXStruct + 'static,
    T: 'static,
{
    type Builder = FXCallback<S, T>;
    type Value = T;

    #[doc(hidden)]
    fn new_default(builder_method: Self::Builder, value: Option<Self::Value>) -> Self {
        Self {
            is_set:  AtomicBool::new(value.is_some()),
            value:   RwLock::new(value),
            builder: RwLock::new(Some(builder_method)),
        }
    }
}

impl<S, T> FXNewDefault for FXProxyAsync<Arc<S>, T>
where
    S: FXStruct,
{
    type Builder = FXCallback<Arc<S>, T>;
    type Value = T;

    #[doc(hidden)]
    fn new_default(builder_method: Self::Builder, value: Option<T>) -> Self {
        Self {
            is_set:  AtomicBool::new(value.is_some()),
            value:   RwLock::new(value),
            builder: RwLock::new(Some(builder_method)),
        }
    }
}

impl<S, T> FXProxyAsync<S, T> {
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
    pub async fn lazy_init<'a>(&'a self, owner: &S) {
        let _ = self.read_or_init(owner).await;
    }

    async fn read_or_init<'a>(&'a self, owner: &S) -> RwLockWriteGuard<'a, Option<T>> {
        let mut guard = self.value.write().await;
        if (*guard).is_none() {
            // No value has been set yet
            match *self.builder.read().await {
                Some(ref builder_cb) => {
                    *guard = Some((*builder_cb)(owner).await);
                    self.is_set_raw().store(true, Ordering::SeqCst);
                }
                None => panic!("Builder is not set"),
            }
        }
        guard
    }

    /// Since the container guarantees that reading from it initializes the wrapped value, this method provides
    /// semit-direct access to it without the [`Option`] wrapper.
    pub async fn read<'a>(&'a self, owner: &S) -> RwLockReadGuard<'a, T> {
        RwLockReadGuard::map(
            RwLockWriteGuard::downgrade(self.read_or_init(owner).await),
            |data: &Option<T>| data.as_ref().unwrap(),
        )
    }

    /// Since the container guarantees that reading from it initializes the wrapped value, this method provides
    /// semit-direct mutable access to it without the [`Option`] wrapper.
    pub async fn read_mut<'a>(&'a self, owner: &S) -> RwLockMappedWriteGuard<'a, T> {
        RwLockWriteGuard::map(self.read_or_init(owner).await, |data: &mut Option<T>| {
            data.as_mut().unwrap()
        })
    }

    /// Provides write-lock to directly store the value.
    pub async fn write<'a>(&'a self) -> FXWrLockGuardAsync<'a, S, T> {
        FXWrLockGuardAsync::<'a, S, T>::new(self.value.write().await, self)
    }

    fn clear_with_lock(&self, wguard: &mut RwLockWriteGuard<Option<T>>) -> Option<T> {
        self.is_set_raw().store(false, Ordering::SeqCst);
        wguard.take()
    }

    /// Resets the container into unitialized state
    pub async fn clear(&self) -> Option<T> {
        let mut wguard = self.value.write().await;
        self.clear_with_lock(&mut wguard)
    }
}

#[allow(private_bounds)]
impl<'a, S, T> FXWrLockGuardAsync<'a, S, T> {
    #[doc(hidden)]
    pub fn new(lock: RwLockWriteGuard<'a, Option<T>>, fxproxy: &'a FXProxyAsync<S, T>) -> Self {
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

impl<S, T> Clone for FXProxyAsync<S, T>
where
    S: FXStruct + Clone + 'static,
    T: Clone + 'static,
    <FXProxyAsync<S, T> as FXNewDefault>::Builder: Clone,
{
    fn clone(&self) -> Self {
        let vguard = self.value.blocking_read();
        let bguard = self.builder.blocking_read();
        Self {
            value:   RwLock::new((*vguard).as_ref().cloned()),
            is_set:  AtomicBool::new(self.is_set()),
            builder: RwLock::new((*bguard).clone()),
        }
    }
}
