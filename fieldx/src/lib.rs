pub mod errors;
pub mod traits;

pub use fieldx_derive::fxstruct;
pub use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard};
use std::{any, cell::RefCell, marker::PhantomData, sync::atomic::AtomicBool};
pub use std::{cell::OnceCell, fmt, sync::atomic::Ordering};
use traits::FXStructSync;

pub struct FXProxy<O, T>
where
    O: FXStructSync,
{
    value: RwLock<Option<T>>,
    is_set: AtomicBool,
    builder: RwLock<Option<fn(&O) -> T>>,
}

#[allow(private_bounds)]
pub struct FXWrLock<'a, O, T, FX>
where
    O: FXStructSync,
    FX: FXProxyCore<O, T>,
{
    lock: RefCell<RwLockWriteGuard<'a, Option<T>>>,
    fxproxy: &'a FX,
    _phantom: PhantomData<O>,
}

impl<O, T> Default for FXProxy<O, T>
where
    O: FXStructSync,
{
    fn default() -> Self {
        Self {
            value: RwLock::new(None),
            is_set: AtomicBool::new(false),
            builder: RwLock::new(None),
        }
    }
}

impl<O, T: fmt::Debug> fmt::Debug for FXProxy<O, T>
where
    O: FXStructSync,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vlock = self.value.read();
        formatter
            .debug_struct(any::type_name::<Self>())
            .field("value", &*vlock)
            .finish()
    }
}

impl<O, T> From<T> for FXProxy<O, T>
where
    O: FXStructSync,
    Self: FXProxyCore<O, T>,
{
    fn from(value: T) -> Self {
        Self {
            value: RwLock::new(Some(value)),
            is_set: AtomicBool::new(true),
            builder: RwLock::new(None),
        }
    }
}

pub trait FXProxyCore<O, T>
where
    O: FXStructSync,
    Self: Sized,
{
    fn builder(&self) -> &RwLock<Option<fn(&O) -> T>>;
    fn value(&self) -> &RwLock<Option<T>>;
    fn is_set_raw(&self) -> &AtomicBool;

    fn is_set(&self) -> bool {
        self.is_set_raw().load(Ordering::SeqCst)
    }

    fn proxy_setup(&self, builder: fn(&O) -> T) {
        *self.builder().write() = Some(builder);
        if !self.is_set() && (*self.value().read()).is_some() {
            self.is_set_raw().store(true, Ordering::SeqCst);
        }
    }

    fn read<'a>(&'a self, owner: &O) -> MappedRwLockReadGuard<'a, T> {
        let guard = self.value().upgradable_read();
        if (*guard).is_none() {
            let mut wguard = RwLockUpgradableReadGuard::upgrade(guard);
            // Still uninitialized? Means no other thread took care of it yet.
            if wguard.is_none() {
                // No value has been set yet
                match *self.builder().read() {
                    Some(ref builder_cb) => {
                        *wguard = Some((*builder_cb)(owner));
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

    fn write<'a>(&'a self) -> FXWrLock<'a, O, T, Self> {
        FXWrLock::<'a, O, T, Self>::new(self.value().write(), self)
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

impl<O, T> FXProxyCore<O, T> for FXProxy<O, T>
where
    O: FXStructSync,
{
    #[inline]
    fn builder(&self) -> &RwLock<Option<fn(&O) -> T>> {
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

impl<O, T> FXProxy<O, T> where O: FXStructSync {}

#[allow(private_bounds)]
impl<'a, O, T, FX> FXWrLock<'a, O, T, FX>
where
    O: FXStructSync,
    FX: FXProxyCore<O, T>,
{
    pub fn new(lock: RwLockWriteGuard<'a, Option<T>>, fxproxy: &'a FX) -> Self
    where
        FX: FXProxyCore<O, T>,
    {
        let lock = RefCell::new(lock);
        Self {
            lock,
            fxproxy,
            _phantom: PhantomData,
        }
    }

    pub fn store(&mut self, value: T) -> Option<T> {
        self.fxproxy.is_set_raw().store(true, Ordering::Release);
        self.lock.borrow_mut().replace(value)
    }

    pub fn clear(&self) -> Option<T> {
        self.fxproxy.clear_with_lock(&mut *self.lock.borrow_mut())
    }
}
