use crate::traits::{FXBuilderWrapper, FXStruct};
use async_trait::async_trait;
use std::{
    any,
    cell::RefCell,
    fmt,
    fmt::{Debug, Formatter},
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
};
use tokio::sync::{RwLock, RwLockMappedWriteGuard, RwLockReadGuard, RwLockWriteGuard};

type FXCallback<S, T> = Box<dyn Fn(&S) -> Pin<Box<dyn Future<Output = T> + Send + '_>> + Send + Sync>;

#[doc(hidden)]
#[async_trait]
pub trait FXBuilderWrapperAsync: FXBuilderWrapper {
    async fn invoke(&self, owner: &Self::Owner) -> Result<Self::Value, Self::Error>;
}

#[doc(hidden)]
pub struct FXBuilderInfallible<S, T> {
    builder: FXCallback<S, T>,
}

impl<S, T> FXBuilderInfallible<S, T> {
    pub fn new(builder: FXCallback<S, T>) -> Self {
        Self { builder }
    }
}

impl<S: FXStruct, T> FXBuilderWrapper for FXBuilderInfallible<S, T> {
    type Error = ();
    type Owner = S;
    type Value = T;
}

#[async_trait]
impl<S, T> FXBuilderWrapperAsync for FXBuilderInfallible<S, T>
where
    S: Sync + FXStruct,
{
    #[inline(always)]
    async fn invoke(&self, owner: &Self::Owner) -> Result<Self::Value, Self::Error> {
        Ok((self.builder)(owner).await)
    }
}

#[doc(hidden)]
pub struct FXBuilderFallible<S, T, E> {
    builder: FXCallback<S, Result<T, E>>,
}

impl<S, T, E> FXBuilderFallible<S, T, E> {
    pub fn new(builder: FXCallback<S, Result<T, E>>) -> Self {
        Self { builder }
    }
}

impl<S, T, E: Debug> FXBuilderWrapper for FXBuilderFallible<S, T, E>
where
    S: FXStruct,
{
    type Error = E;
    type Owner = S;
    type Value = T;
}

#[async_trait]
impl<S, T, E: Debug> FXBuilderWrapperAsync for FXBuilderFallible<S, T, E>
where
    S: FXStruct + Sync,
{
    #[inline(always)]
    async fn invoke(&self, owner: &Self::Owner) -> Result<Self::Value, Self::Error> {
        (self.builder)(owner).await
    }
}

/// Container type for lazy fields
pub struct FXProxyAsync<B>
where
    B: FXBuilderWrapperAsync,
{
    value:   RwLock<Option<B::Value>>,
    is_set:  AtomicBool,
    builder: RwLock<Option<B>>,
}

/// Write-lock returned by [`FXProxyAsync::write`] method
///
/// This type, in cooperation with the [`FXProxyAsync`] type, takes care of safely updating lazy field status when data
/// is being stored.
pub struct FXWrLockGuardAsync<'a, B>
where
    B: FXBuilderWrapperAsync,
{
    lock:    RefCell<RwLockWriteGuard<'a, Option<B::Value>>>,
    fxproxy: &'a FXProxyAsync<B>,
}

impl<B, V> Debug for FXProxyAsync<B>
where
    B: FXBuilderWrapperAsync<Value = V>,
    V: Debug,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let vlock = self.value.blocking_read();
        formatter
            .debug_struct(any::type_name::<Self>())
            .field("value", &*vlock)
            .finish()
    }
}

impl<B, E> FXProxyAsync<B>
where
    B: FXBuilderWrapperAsync<Error = E>,
    E: Debug,
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
    pub async fn lazy_init<'a>(&'a self, owner: &B::Owner) {
        let _ = self.read_or_init(owner).await;
    }

    async fn read_or_init<'a>(&'a self, owner: &B::Owner) -> Result<RwLockWriteGuard<'a, Option<B::Value>>, B::Error> {
        let mut guard = self.value.write().await;
        if (*guard).is_none() {
            // No value has been set yet
            match *self.builder.read().await {
                Some(ref builder_cb) => {
                    *guard = Some((*builder_cb).invoke(owner).await?);
                    self.is_set_raw().store(true, Ordering::SeqCst);
                }
                None => panic!("Builder is not set"),
            }
        }
        Ok(guard)
    }

    /// Lazy-initialize the field if necessary and return lock read guard for the inner value.
    ///
    /// Panics if fallible field builder returns an error.
    pub async fn read<'a>(&'a self, owner: &B::Owner) -> RwLockReadGuard<'a, B::Value> {
        RwLockReadGuard::map(
            RwLockWriteGuard::downgrade(self.read_or_init(owner).await.unwrap()),
            |data: &Option<B::Value>| data.as_ref().unwrap(),
        )
    }

    /// Lazy-initialize the field if necessary and return lock write guard for the inner value.
    ///
    /// Panics if fallible field builder returns an error.
    pub async fn read_mut<'a>(&'a self, owner: &B::Owner) -> RwLockMappedWriteGuard<'a, B::Value> {
        RwLockWriteGuard::map(
            self.read_or_init(owner).await.unwrap(),
            |data: &mut Option<B::Value>| data.as_mut().unwrap(),
        )
    }

    /// Lazy-initialize the field if necessary and return lock read guard for the inner value.
    ///
    /// Return the same error, as fallible field builder if it errors out.
    pub async fn try_read<'a>(&'a self, owner: &B::Owner) -> Result<RwLockReadGuard<'a, B::Value>, B::Error> {
        Ok(RwLockReadGuard::map(
            RwLockWriteGuard::downgrade(self.read_or_init(owner).await?),
            |data: &Option<B::Value>| data.as_ref().unwrap(),
        ))
    }

    /// Lazy-initialize the field if necessary and return lock write guard for the inner value.
    ///
    /// Return the same error, as fallible field builder if it errors out.
    pub async fn try_read_mut<'a>(
        &'a self,
        owner: &B::Owner,
    ) -> Result<RwLockMappedWriteGuard<'a, B::Value>, B::Error> {
        Ok(RwLockWriteGuard::map(
            self.read_or_init(owner).await?,
            |data: &mut Option<B::Value>| data.as_mut().unwrap(),
        ))
    }

    /// Provides write-lock to directly store the value. Never calls the lazy builder.
    pub async fn write<'a>(&'a self) -> FXWrLockGuardAsync<'a, B> {
        FXWrLockGuardAsync::<'a, B>::new(self.value.write().await, self)
    }

    fn clear_with_lock(&self, wguard: &mut RwLockWriteGuard<Option<B::Value>>) -> Option<B::Value> {
        self.is_set_raw().store(false, Ordering::SeqCst);
        wguard.take()
    }

    /// Resets the container into unitialized state
    pub async fn clear(&self) -> Option<B::Value> {
        let mut wguard = self.value.write().await;
        self.clear_with_lock(&mut wguard)
    }
}

#[allow(private_bounds)]
impl<'a, B> FXWrLockGuardAsync<'a, B>
where
    B: FXBuilderWrapperAsync,
{
    #[doc(hidden)]
    pub fn new(lock: RwLockWriteGuard<'a, Option<B::Value>>, fxproxy: &'a FXProxyAsync<B>) -> Self {
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

impl<B> Clone for FXProxyAsync<B>
where
    B: FXBuilderWrapperAsync + Clone,
    B::Value: Clone,
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
