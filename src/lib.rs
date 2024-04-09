pub use fieldx_derive::fxstruct;
pub use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard};
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

pub struct FXWrLock<'a, T> {
    lock:    RefCell<RwLockWriteGuard<'a, Option<T>>>,
    fxproxy: &'a FXProxy<T>,
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

impl<T> From<Option<T>> for FXProxy<T> {
    fn from(value: Option<T>) -> Self {
        Self {
            value:   RwLock::new(value),
            is_set:  AtomicBool::new(true),
            builder: RwLock::new(None),
        }
    }
}

impl<T> FXProxy<T> {
    pub fn proxy_setup(&self, builder: Box<dyn Fn() -> T + Send + Sync>) {
        *self.builder.write() = Some(builder);
        if !self.is_set() && (*self.value.read()).is_some() {
            self.is_set.store(true, Ordering::SeqCst);
        }
    }

    pub fn read<'a>(&'a self) -> MappedRwLockReadGuard<'a, T> {
        let guard = self.value.upgradable_read();
        if (*guard).is_none() {
            // eprintln!("+ need to rebuild");
            let mut wguard = RwLockUpgradableReadGuard::upgrade(guard);
            // eprintln!("+ write lock obtained");
            // Still uninitialized? Means no other thread took care of it yet.
            if wguard.is_none() {
                // No value has been set yet
                match *self.builder.read() {
                    Some(ref builder_cb) => {
                        *wguard = Some((*builder_cb)());
                        self.is_set.store(true, Ordering::SeqCst);
                        // eprintln!("+ done building");
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

    pub fn write<'a>(&'a self) -> FXWrLock<'a, T> {
        FXWrLock::<'a, T>::new(self.value.write(), self)
    }

    fn clear_with_lock(&self, wguard: &mut RwLockWriteGuard<Option<T>>) -> Option<T> {
        self.is_set.store(false, Ordering::SeqCst);
        wguard.take()
    }

    pub fn clear(&self) -> Option<T> {
        let mut wguard = self.value.write();
        self.clear_with_lock(&mut wguard)
    }

    pub fn is_set(&self) -> bool {
        self.is_set.load(Ordering::SeqCst)
    }
}

impl<'a, T> FXWrLock<'a, T> {
    pub fn new(lock: RwLockWriteGuard<'a, Option<T>>, fxproxy: &'a FXProxy<T>) -> Self {
        let lock = RefCell::new(lock);
        Self { lock, fxproxy }
    }

    pub fn store(&mut self, value: T) -> Option<T> {
        self.fxproxy.is_set.store(true, Ordering::Release);
        self.lock.borrow_mut().replace(value)
    }

    pub fn clear(&self) -> Option<T> {
        self.fxproxy.clear_with_lock(&mut *self.lock.borrow_mut())
    }
}
