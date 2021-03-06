// Copyright (C) 2017 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

//! An immutable memory location that implements `Send` for types that do not implement it

extern crate fragile;

use std::cmp;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops;

/// An immutable memory location that implements `Send` for types that do not implement it
///
/// Enforcing safety with regard to the `Send` trait happens at runtime instead of compile time.
/// Accessing the contained value will call `panic!` if happening from any thread but the thread on
/// which the value was created on. The `SendCell` can be safely transferred to other threads.
///
/// # Warning
///
/// Any other usage from a different thread will lead to a panic, i.e. using any of the traits
/// implemented on `SendCell` like `Eq`.
///
/// Calling `drop` on a `SendCell` or otherwise freeing the value from a different thread than the
/// one where it was created also results in a panic.
pub struct SendCell<T> {
    value: fragile::Fragile<T>,
}

impl<T> SendCell<T> {
    /// Creates a new `SendCell` containing `value`.
    pub fn new(value: T) -> Self {
        SendCell {
            value: fragile::Fragile::new(value),
        }
    }

    /// Consumes the `SendCell`, returning the wrapped value.
    ///
    /// # Panics
    ///
    /// Panics if called from a different thread than the one where the original value was created.
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }

    /// Consumes the `SendCell`, returning the wrapped value if successful.
    ///
    /// The wrapped value is returned if this is called from the same thread as the one where the
    /// original value was created, otherwise the `SendCell` is returned as `Err(self)`.
    pub fn try_into_inner(self) -> Result<T, Self> {
        self.value
            .try_into_inner()
            .map_err(|v| SendCell { value: v })
    }

    /// Immutably borrows the wrapped value.
    ///
    /// Multiple immutable borrows can be taken out at the same time.
    ///
    /// # Panics
    ///
    /// Panics if called from a different thread than the one where the original value was created.
    pub fn get(&self) -> &T {
        self.value.get()
    }

    /// Tries to immutably borrow the wrapped value.
    ///
    /// `None` is returned if called from a different thread than the one where the original value
    /// was created.
    ///
    /// Multiple immutable borrows can be taken out at the same time.
    pub fn try_get(&self) -> Option<&T> {
        self.value.try_get().ok()
    }

    /// Immutably borrows the wrapped value.
    ///
    /// The borrow lasts until the returned `Ref` exits scope. Multiple immutable borrows can be
    /// taken out at the same time.
    ///
    /// # Panics
    ///
    /// Panics if called from a different thread than the one where the original value was created.
    pub fn borrow(&self) -> Ref<T> {
        Ref { value: self.get() }
    }

    /// Tries to immutably borrow the wrapped value.
    ///
    /// `None` is returned if called from a different thread than the one where the original value
    /// was created.
    ///
    /// The borrow lasts until the returned `Ref` exits scope. Multiple immutable borrows can be
    /// taken out at the same time.
    pub fn try_borrow(&self) -> Option<Ref<T>> {
        self.try_get().map(|value| Ref { value: value })
    }
}

impl<T> From<T> for SendCell<T> {
    fn from(t: T) -> SendCell<T> {
        SendCell::new(t)
    }
}

impl<T: Default> Default for SendCell<T> {
    fn default() -> SendCell<T> {
        SendCell::new(T::default())
    }
}

impl<T: Clone> Clone for SendCell<T> {
    fn clone(&self) -> SendCell<T> {
        SendCell::new(self.get().clone())
    }
}

impl<T: fmt::Debug> fmt::Debug for SendCell<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.get().fmt(f)
    }
}

impl<T: PartialEq> PartialEq<SendCell<T>> for SendCell<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get().eq(other.get())
    }
}
impl<T: Eq> Eq for SendCell<T> {}

impl<T: PartialOrd> PartialOrd<SendCell<T>> for SendCell<T> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.get().partial_cmp(other.get())
    }
}
impl<T: Ord> Ord for SendCell<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.get().cmp(other.get())
    }
}

impl<T: Hash> Hash for SendCell<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.get().hash(state)
    }
}

unsafe impl<T> Send for SendCell<T> {}

/// Wraps a borrowed reference to a value in a `SendCell` box.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ref<'a, T: 'a> {
    value: &'a T,
}

impl<'a, T: 'a> ops::Deref for Ref<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;
    use std::panic;
    use std::thread;

    #[test]
    fn get_success() {
        let cell = SendCell::new(1);
        assert_eq!(cell.get(), &1);
        assert_eq!(cell.try_get(), Some(&1));
    }

    #[test]
    #[should_panic]
    fn get_failure() {
        let t = thread::spawn(move || {
            let cell = SendCell::new(1);
            assert_eq!(cell.get(), &1);
            cell
        });

        let r = t.join();
        let cell = r.unwrap();

        // Some dance here to
        // a) Not panic uncontrolled: this would run the Drop impl
        //    of SendCell from the wrong thread, which panics again
        // b) Make the borrow checker happy so that we can forget
        //    the cell in case of panic (and don't have it borrowed anymore)
        // c) And then rethrow the panic
        let panic = {
            let res = panic::catch_unwind(panic::AssertUnwindSafe(|| cell.get()));

            res.err()
        };
        mem::forget(cell);
        if let Some(payload) = panic {
            panic::resume_unwind(payload);
        }
    }

    #[test]
    fn try_get_failure() {
        let t = thread::spawn(move || {
            let cell = SendCell::new(1);
            assert_eq!(cell.get(), &1);
            cell
        });

        let r = t.join();
        let cell = r.unwrap();

        assert_eq!(cell.try_get(), None);
        // Forget so drop() is not run, which would panic
        mem::forget(cell);
    }

    #[test]
    fn borrow_success() {
        let cell = SendCell::new(1);
        assert_eq!(*cell.borrow(), 1);
        assert_eq!(*cell.try_borrow().unwrap(), 1);
    }

    #[test]
    #[should_panic]
    fn borrow_failure() {
        let t = thread::spawn(move || {
            let cell = SendCell::new(1);
            assert_eq!(*cell.borrow(), 1);
            cell
        });

        let r = t.join();
        let cell = r.unwrap();

        // Some dance here to
        // a) Not panic uncontrolled: this would run the Drop impl
        //    of SendCell from the wrong thread, which panics again
        // b) Make the borrow checker happy so that we can forget
        //    the cell in case of panic (and don't have it borrowed anymore)
        // c) And then rethrow the panic
        let panic = {
            let res = panic::catch_unwind(panic::AssertUnwindSafe(|| cell.borrow()));

            res.err()
        };
        mem::forget(cell);
        if let Some(payload) = panic {
            panic::resume_unwind(payload);
        }
    }

    #[test]
    fn try_borrow_failure() {
        let t = thread::spawn(move || {
            let cell = SendCell::new(1);
            assert_eq!(*cell.borrow(), 1);
            cell
        });

        let r = t.join();
        let cell = r.unwrap();

        assert_eq!(cell.try_borrow(), None);
        // Forget so drop() is not run, which would panic
        mem::forget(cell);
    }

    #[test]
    fn into_inner_success() {
        let cell = SendCell::new(1);
        assert_eq!(cell.try_into_inner().unwrap(), 1);
    }

    // FIXME: Can't test the failure case of to_inner() as it will
    // panic and then during unwinding call the Drop impl of SendCell,
    // which will panic again and can't be handled by the test

    #[test]
    fn try_into_inner_failure() {
        let t = thread::spawn(move || SendCell::new(1));

        let r = t.join();
        let cell = r.unwrap();

        let res = cell.try_into_inner();
        assert!(res.is_err());
        // Forget so drop() is not run, which would panic
        mem::forget(res);
    }

    struct Dummy(i32);
    impl Drop for Dummy {
        fn drop(&mut self) {}
    }

    #[test]
    #[should_panic]
    fn drop_panic() {
        let t = thread::spawn(move || SendCell::new(Dummy(1)));

        let r = t.join();
        let _ = r.unwrap();
    }

    #[test]
    fn drop_is_not_run_from_other_thread() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        struct MakeItTrueOnDrop(Arc<AtomicBool>);

        impl Drop for MakeItTrueOnDrop {
            fn drop(&mut self) {
                self.0.swap(true, Ordering::SeqCst);
            }
        }

        let is_dropped = Arc::new(AtomicBool::new(false));
        let v = SendCell::new(MakeItTrueOnDrop(is_dropped.clone()));
        let t = thread::spawn(move || {
            let _ = v;
        });
        let error = t.join().expect_err("thread should have panicked");
        assert_eq!(
            error.downcast_ref::<&str>(),
            Some(&"destructor of fragile object ran on wrong thread")
        );
        assert_eq!(
            is_dropped.load(Ordering::SeqCst),
            false,
            "Drop impl should not have been executed"
        );
    }
}
