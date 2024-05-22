pub mod errors;
pub mod traits;

pub use fieldx_derive::fxstruct;
pub use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard};
use std::{any, borrow::Borrow, cell::RefCell, fmt::Debug, marker::PhantomData, ops::Deref, sync::atomic::AtomicBool};
pub use std::{cell::OnceCell, fmt, sync::atomic::Ordering};
use traits::FXStructSync;

pub struct FXProxy<O, T>
where
    O: FXStructSync,
{
    value:   RwLock<Option<T>>,
    is_set:  AtomicBool,
    builder: RwLock<Option<fn(&O) -> T>>,
}

// We need FXRwLock because RwLock doesn't implement Clone
#[derive(Default)]
pub struct FXRwLock<T>(RwLock<T>);

#[allow(private_bounds)]
pub struct FXWrLock<'a, O, T>
where
    O: FXStructSync,
{
    lock:     RefCell<RwLockWriteGuard<'a, Option<T>>>,
    fxproxy:  &'a FXProxy<O, T>,
    _phantom: PhantomData<O>,
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

impl<O, T> From<(fn(&O) -> T, Option<T>)> for FXProxy<O, T>
where
    O: FXStructSync,
{
    fn from((builder_method, value): (fn(&O) -> T, Option<T>)) -> Self {
        Self::new_default(builder_method, value)
    }
}

impl<O, T> FXProxy<O, T>
where
    O: FXStructSync,
{
    pub fn new_default(builder_method: fn(&O) -> T, value: Option<T>) -> Self {
        Self {
            is_set:  AtomicBool::new(value.is_some()),
            value:   RwLock::new(value),
            builder: RwLock::new(Some(builder_method)),
        }
    }

    pub fn into_inner(self) -> Option<T> {
        self.value.into_inner()
    }

    #[inline]
    fn is_set_raw(&self) -> &AtomicBool {
        &self.is_set
    }

    pub fn is_set(&self) -> bool {
        self.is_set_raw().load(Ordering::SeqCst)
    }

    pub fn read_or_init<'a>(&'a self, owner: &O) -> RwLockReadGuard<'a, Option<T>> {
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
            RwLockWriteGuard::downgrade(wguard)
        }
        else {
            RwLockUpgradableReadGuard::downgrade(guard)
        }
    }

    pub fn read<'a>(&'a self, owner: &O) -> MappedRwLockReadGuard<'a, T> {
        RwLockReadGuard::map(self.read_or_init(owner), |data: &Option<T>| data.as_ref().unwrap())
    }

    pub fn write<'a>(&'a self) -> FXWrLock<'a, O, T> {
        FXWrLock::<'a, O, T>::new(self.value.write(), self)
    }

    fn clear_with_lock(&self, wguard: &mut RwLockWriteGuard<Option<T>>) -> Option<T> {
        self.is_set_raw().store(false, Ordering::SeqCst);
        wguard.take()
    }

    pub fn clear(&self) -> Option<T> {
        let mut wguard = self.value.write();
        self.clear_with_lock(&mut wguard)
    }
}

#[allow(private_bounds)]
impl<'a, O, T> FXWrLock<'a, O, T>
where
    O: FXStructSync,
{
    pub fn new(lock: RwLockWriteGuard<'a, Option<T>>, fxproxy: &'a FXProxy<O, T>) -> Self {
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

impl<O, T> Clone for FXProxy<O, T>
where
    O: FXStructSync,
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

impl<T> FXRwLock<T> {
    pub fn new(value: T) -> Self {
        Self(RwLock::new(value))
    }

    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }
}

impl<T> From<T> for FXRwLock<T> {
    fn from(value: T) -> Self {
        Self(RwLock::new(value.into()))
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
