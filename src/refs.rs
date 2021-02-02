#[cfg(not(feature = "parallel"))]
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use std::sync::RwLock as RefCell;

#[cfg(not(feature = "parallel"))]
pub use std::{
    cell::{Ref as ReadRef, RefMut as WriteRef},
    rc::{Rc as Ref, Weak},
};

#[cfg(feature = "parallel")]
pub use std::sync::{Arc as Ref, RwLockReadGuard as ReadRef, RwLockWriteGuard as WriteRef, Weak};

/// The marker trait which requires [`Send`] when `"parallel"` feature is used
#[cfg(not(feature = "parallel"))]
pub trait ParallelSend {}

#[cfg(feature = "parallel")]
pub trait ParallelSend: Send {}

#[cfg(not(feature = "parallel"))]
impl<T> ParallelSend for T {}

#[cfg(feature = "parallel")]
impl<T: Send> ParallelSend for T {}

/// The marker trait which requires [`Sync`] when `"parallel"` feature is used
#[cfg(not(feature = "parallel"))]
pub trait ParallelSync {}

#[cfg(feature = "parallel")]
pub trait ParallelSync: Sync {}

#[cfg(not(feature = "parallel"))]
impl<T> ParallelSync for T {}

#[cfg(feature = "parallel")]
impl<T: Sync> ParallelSync for T {}

#[repr(transparent)]
pub struct Mut<T: ?Sized>(RefCell<T>);

impl<T> Mut<T> {
    pub fn new(inner: T) -> Self {
        Self(RefCell::new(inner))
    }
}

impl<T: Default> Default for Mut<T> {
    fn default() -> Self {
        Mut::new(T::default())
    }
}

impl<T: ?Sized> Mut<T> {
    pub fn read(&self) -> ReadRef<T> {
        #[cfg(not(feature = "parallel"))]
        {
            self.0.borrow()
        }

        #[cfg(feature = "parallel")]
        {
            self.0.read().unwrap()
        }
    }

    pub fn write(&self) -> WriteRef<T> {
        #[cfg(not(feature = "parallel"))]
        {
            self.0.borrow_mut()
        }

        #[cfg(feature = "parallel")]
        {
            self.0.write().unwrap()
        }
    }

    pub fn try_read(&self) -> Option<ReadRef<T>> {
        #[cfg(not(feature = "parallel"))]
        {
            self.0.try_borrow().ok()
        }

        #[cfg(feature = "parallel")]
        {
            self.0.read().ok()
        }
    }

    pub fn try_write(&self) -> Option<WriteRef<T>> {
        #[cfg(not(feature = "parallel"))]
        {
            self.0.try_borrow_mut().ok()
        }

        #[cfg(feature = "parallel")]
        {
            self.0.write().ok()
        }
    }
}
