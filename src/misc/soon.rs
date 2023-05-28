// On the off chance that somebody is actually looking at this.
// DO NOT USE IT. THERE IS NO GOOD REASON TO DO SOMETHING LIKE THIS.
// I only wrote this because my reasons are bad.

use std::{cell::UnsafeCell, mem::MaybeUninit, ops::Deref};

/// A *VERY UNSAFE* way to set values after creating a struct.
/// Like a RefCell without the borrow checking.
/// You are expected to use it properly.
pub struct Soon<T> {
    inner: MaybeUninit<UnsafeCell<T>>,
}

impl<T> Soon<T> {
    /// Create a new `Soon` with out its value.
    /// If it is dereferenced at this point, in debug mode it will panic
    /// and in release mode you will get some sorta segfault.
    /// **(very unsafe)**
    pub fn empty() -> Self {
        Self {
            inner: MaybeUninit::zeroed(),
        }
    }

    /// Replace whatever is in the `Soon` with a specified value.
    /// Please only call this once per soon object.
    pub fn replace(&self, val: T) {
        let cell = UnsafeCell::raw_get(self.inner.as_ptr());
        // SAFETY: nobody cares
        unsafe {
            cell.write(val);
        }
    }
}

impl<T> Deref for Soon<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let cell = UnsafeCell::raw_get(self.inner.as_ptr());
        let data = unsafe { cell.as_ref() };
        debug_assert!(
            data.is_some(),
            "A `Soon` was dereferenced before being givin a value."
        );

        data.unwrap()
    }
}

// shhhhh. its not really thread safe.
unsafe impl<T> Send for Soon<T> {}
unsafe impl<T> Sync for Soon<T> {}
