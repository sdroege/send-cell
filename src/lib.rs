// Copyright (C) 2017 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use std::thread;
use std::fmt;
use std::cmp;
use std::ops;
use std::hash::{Hash, Hasher};

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
    value: Option<T>,
    thread_id: thread::ThreadId,
}

impl<T> SendCell<T> {
    /// Creates a new `SendCell` containing `value`.
    pub fn new(value: T) -> Self {
        SendCell {
            value: Some(value),
            thread_id: thread::current().id(),
        }
    }

    /// Consumes the `SendCell`, returning the wrapped value.
    ///
    /// # Panics
    ///
    /// Panics if called from a different thread than the one where the original value was created.
    pub fn into_inner(mut self) -> T {
        if thread::current().id() != self.thread_id {
            panic!("trying to convert to inner value in invalid thread");
        }

        self.value.take().unwrap()
    }

    /// Consumes the `SendCell`, returning the wrapped value if successful.
    ///
    /// The wrapped value is returned if this is called from the same thread as the one where the
    /// original value was created, otherwise the `SendCell` is returned as `Err(self)`.
    pub fn try_into_inner(mut self) -> Result<T, Self> {
        if thread::current().id() == self.thread_id {
            Ok(self.value.take().unwrap())
        } else {
            Err(self)
        }
    }

    /// Immutably borrows the wrapped value.
    ///
    /// Multiple immutable borrows can be taken out at the same time.
    ///
    /// # Panics
    ///
    /// Panics if called from a different thread than the one where the original value was created.
    pub fn get(&self) -> &T {
        if thread::current().id() != self.thread_id {
            panic!("trying to convert to inner value in invalid thread");
        }

        self.value.as_ref().unwrap()
    }

    /// Tries to immutably borrow the wrapped value.
    ///
    /// `None` is returned if called from a different thread than the one where the original value
    /// was created.
    ///
    /// Multiple immutable borrows can be taken out at the same time.
    pub fn try_get(&self) -> Option<&T> {
        if thread::current().id() == self.thread_id {
            Some(self.value.as_ref().unwrap())
        } else {
            None
        }
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

impl<T> Drop for SendCell<T> {
    fn drop(&mut self) {
        if thread::current().id() != self.thread_id {
            panic!("trying to convert to inner value in invalid thread");
        }
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
    use std::thread;

    #[test]
    fn get_success() {
        let cell = SendCell::new(1);
        assert_eq!(cell.get(), &1);
        assert_eq!(cell.try_get(), Some(&1));
    }

    // FIXME: How to test this? This will panic, and
    // then during panicking the destructor will panic!
    //#[test]
    //#[should_panic]
    //fn get_failure() {
    //    let t = thread::spawn(move || {
    //        let cell = SendCell::new(1);
    //        assert_eq!(cell.get(), &1);
    //        cell
    //    });
    //
    //    let r = t.join();
    //    let cell = r.unwrap();
    //
    //    let _ = cell.get();
    //}

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

    // FIXME: How to test this? This will panic, and
    // then during panicking the destructor will panic!
    //#[test]
    //#[should_panic]
    //fn borrow_failure() {
    //    let t = thread::spawn(move || {
    //        let cell = SendCell::new(1);
    //        assert_eq!(*cell.borrow(), 1);
    //        cell
    //    });
    //
    //    let r = t.join();
    //    let cell = r.unwrap();
    //
    //    let _ = cell.borrow();
    //}

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

    // FIXME: How to test this? This will panic, and
    // then during panicking the destructor will panic!
    //#[test]
    //#[should_panic]
    //fn into_inner_failure() {
    //    let t = thread::spawn(move || SendCell::new(1));
    //
    //    let r = t.join();
    //    let cell = r.unwrap();
    //
    //    let _ = cell.into_inner();
    //}

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

    #[test]
    #[should_panic]
    fn drop_panic() {
        let t = thread::spawn(move || SendCell::new(1));

        let r = t.join();
        let _ = r.unwrap();
    }
}
