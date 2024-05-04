pub use fieldx_derive::fxstruct;
pub use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::{any, cell::RefCell, sync::atomic::AtomicBool};
pub use std::{
    cell::OnceCell,
    fmt,
    sync::{atomic::Ordering, Arc},
};

pub mod errors;
pub mod traits;

pub struct FXProxy<T> {
    value:   RwLock<Option<T>>,
    is_set:  AtomicBool,
    builder: RwLock<Option<Box<dyn Fn() -> T + Send + Sync>>>,
}

#[cfg(feature = "serde")]
pub struct FXProxySerde<T> {
    value:   RwLock<Option<T>>,
    is_set:  AtomicBool,
    builder: RwLock<Option<Box<dyn Fn() -> T + Send + Sync>>>,
}

#[allow(private_bounds)]
pub struct FXWrLock<'a, T, FX>
where
    FX: FXProxyCore<T>,
{
    lock:    RefCell<RwLockWriteGuard<'a, Option<T>>>,
    fxproxy: &'a FX,
}

impl<T> Default for FXProxy<T> {
    fn default() -> Self {
        Self {
            value:   RwLock::new(None),
            is_set:  AtomicBool::new(false),
            builder: RwLock::new(None),
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for FXProxy<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vlock = self.value.read();
        formatter
            .debug_struct(any::type_name::<Self>())
            .field("value", &*vlock)
            .finish()
    }
}

impl<T> From<T> for FXProxy<T> {
    fn from(value: T) -> Self {
        Self {
            value:   RwLock::new(Some(value)),
            is_set:  AtomicBool::new(true),
            builder: RwLock::new(None),
        }
    }
}

pub trait FXProxyCore<T>
where
    Self: Sized,
{
    fn builder(&self) -> &RwLock<Option<Box<dyn Fn() -> T + Send + Sync>>>;
    fn value(&self) -> &RwLock<Option<T>>;
    fn is_set_raw(&self) -> &AtomicBool;

    fn is_set(&self) -> bool {
        self.is_set_raw().load(Ordering::SeqCst)
    }

    fn proxy_setup(&self, builder: Box<dyn Fn() -> T + Send + Sync>) {
        *self.builder().write() = Some(builder);
        if !self.is_set() && (*self.value().read()).is_some() {
            self.is_set_raw().store(true, Ordering::SeqCst);
        }
    }

    fn read<'a>(&'a self) -> MappedRwLockReadGuard<'a, T> {
        let guard = self.value().upgradable_read();
        if (*guard).is_none() {
            let mut wguard = RwLockUpgradableReadGuard::upgrade(guard);
            // Still uninitialized? Means no other thread took care of it yet.
            if wguard.is_none() {
                // No value has been set yet
                match *self.builder().read() {
                    Some(ref builder_cb) => {
                        *wguard = Some((*builder_cb)());
                        self.is_set_raw().store(true, Ordering::SeqCst);
                    }
                    None => panic!("Builder is not set"),
                }
            }
            return RwLockReadGuard::map(RwLockWriteGuard::downgrade(wguard), |data: &Option<T>| {
                data.as_ref().unwrap()
            });
        }
        RwLockReadGuard::map(RwLockUpgradableReadGuard::downgrade(guard), |data: &Option<T>| {
            data.as_ref().unwrap()
        })
    }

    fn write<'a>(&'a self) -> FXWrLock<'a, T, Self> {
        FXWrLock::<'a, T, Self>::new(self.value().write(), self)
    }

    fn clear_with_lock(&self, wguard: &mut RwLockWriteGuard<Option<T>>) -> Option<T> {
        self.is_set_raw().store(false, Ordering::SeqCst);
        wguard.take()
    }

    fn clear(&self) -> Option<T> {
        let mut wguard = self.value().write();
        self.clear_with_lock(&mut wguard)
    }
}

impl<T> FXProxyCore<T> for FXProxy<T> {
    #[inline]
    fn builder(&self) -> &RwLock<Option<Box<dyn Fn() -> T + Send + Sync>>> {
        &self.builder
    }

    #[inline]
    fn value(&self) -> &RwLock<Option<T>> {
        &self.value
    }

    #[inline]
    fn is_set_raw(&self) -> &AtomicBool {
        &self.is_set
    }
}

impl<T> FXProxy<T> {}

#[allow(private_bounds)]
impl<'a, T, FX> FXWrLock<'a, T, FX>
where
    FX: FXProxyCore<T>,
{
    pub fn new(lock: RwLockWriteGuard<'a, Option<T>>, fxproxy: &'a FX) -> Self
    where
        FX: FXProxyCore<T>,
    {
        let lock = RefCell::new(lock);
        Self { lock, fxproxy }
    }

    pub fn store(&mut self, value: T) -> Option<T> {
        self.fxproxy.is_set_raw().store(true, Ordering::Release);
        self.lock.borrow_mut().replace(value)
    }

    pub fn clear(&self) -> Option<T> {
        self.fxproxy.clear_with_lock(&mut *self.lock.borrow_mut())
    }
}

#[cfg(feature = "serde")]
const _: () = {
    impl<T> Default for FXProxySerde<T> {
        fn default() -> Self {
            Self {
                value:   RwLock::new(None),
                is_set:  AtomicBool::new(false),
                builder: RwLock::new(None),
            }
        }
    }
    impl<T: fmt::Debug> fmt::Debug for FXProxySerde<T> {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            let vlock = self.value.read();
            formatter
                .debug_struct(any::type_name::<Self>())
                .field("value", &*vlock)
                .finish()
        }
    }

    impl<T> From<T> for FXProxySerde<T> {
        fn from(value: T) -> Self {
            Self {
                value:   RwLock::new(Some(value)),
                is_set:  AtomicBool::new(true),
                builder: RwLock::new(None),
            }
        }
    }

    impl<T> FXProxyCore<T> for FXProxySerde<T> {
        #[inline]
        fn builder(&self) -> &RwLock<Option<Box<dyn Fn() -> T + Send + Sync>>> {
            &self.builder
        }

        #[inline]
        fn value(&self) -> &RwLock<Option<T>> {
            &self.value
        }

        #[inline]
        fn is_set_raw(&self) -> &AtomicBool {
            &self.is_set
        }
    }

    impl<'de, T> Deserialize<'de> for FXProxySerde<T>
    where
        T: Deserialize<'de>,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let value = T::deserialize(deserializer)?;
            Ok(Self::from(value))
        }
    }

    impl<T> Serialize for FXProxySerde<T>
    where
        T: Serialize,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let value = self.read();
            (*value).serialize(serializer)
        }
    }
};
