#[cfg(not(feature = "parallel"))]
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use std::sync::{Arc as Ref, RwLock as RefCell, Weak};

#[cfg(not(feature = "parallel"))]
pub use std::{
    cell::{Ref as ReadRef, RefMut as WriteRef},
    rc::{Rc as Ref, Weak},
};

#[cfg(feature = "parallel")]
pub use std::sync::{Arc as Ref, RwLockReadGuard as ReadRef, RwLockWriteGuard as WriteRef, Weak};

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
