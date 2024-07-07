use core::mem::MaybeUninit;
use core::slice;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{
    alloc::{Allocator, Global},
    vec::Vec,
};
#[cfg(feature = "std")]
use std::{
    alloc::{Allocator, Global},
    vec::Vec,
};

use wrapper::Wrapper;

use crate::maybe_uninit_slice_mut;
use crate::sync::consumer::Invariant;
use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

/// Collects data and can at any point be converted into a `Vec<T>`.
pub struct IntoVec_<T, A: Allocator = Global>(Invariant<IntoVec<T, A>>);

impl<T: core::fmt::Debug, A: Allocator + core::fmt::Debug> core::fmt::Debug for IntoVec_<T, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> Default for IntoVec_<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntoVec_<T> {
    pub fn new() -> IntoVec_<T> {
        let invariant = Invariant::new(IntoVec(Vec::new()));

        IntoVec_(invariant)
    }

    pub fn into_vec(self) -> Vec<T> {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<T, A: Allocator> IntoVec_<T, A> {
    pub fn new_in(alloc: A) -> IntoVec_<T, A> {
        let invariant = Invariant::new(IntoVec(Vec::new_in(alloc)));

        IntoVec_(invariant)
    }
}

impl<T> AsRef<Vec<T>> for IntoVec_<T> {
    fn as_ref(&self) -> &Vec<T> {
        let inner = self.0.as_ref();
        inner.as_ref()
    }
}

impl<T> AsMut<Vec<T>> for IntoVec_<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        let inner = self.0.as_mut();
        inner.as_mut()
    }
}

impl<T> Wrapper<Vec<T>> for IntoVec_<T> {
    fn into_inner(self) -> Vec<T> {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<T> Consumer for IntoVec_<T> {
    type Item = T;
    type Final = ();
    type Error = !;

    fn consume(&mut self, item: T) -> Result<(), Self::Error> {
        self.0.consume(item)
    }

    fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        self.0.close(final_val)
    }
}

impl<T> BufferedConsumer for IntoVec_<T> {
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush()
    }
}

impl<T: Copy> BulkConsumer for IntoVec_<T> {
    fn expose_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        self.0.expose_slots()
    }

    unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.consume_slots(amount)
    }
}

#[derive(Debug)]
struct IntoVec<T, A: Allocator = Global>(Vec<T, A>);

impl<T> AsRef<Vec<T>> for IntoVec<T> {
    fn as_ref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T> AsMut<Vec<T>> for IntoVec<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}

impl<T> Wrapper<Vec<T>> for IntoVec<T> {
    fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<T> Consumer for IntoVec<T> {
    type Item = T;
    type Final = ();
    type Error = !;

    fn consume(&mut self, item: T) -> Result<Self::Final, Self::Error> {
        self.0.push(item);

        Ok(())
    }

    fn close(&mut self, _final: Self::Final) -> Result<Self::Final, Self::Error> {
        Ok(())
    }
}

impl<T> BufferedConsumer for IntoVec<T> {
    fn flush(&mut self) -> Result<Self::Final, Self::Error> {
        Ok(())
    }
}

impl<T: Copy> BulkConsumer for IntoVec<T> {
    fn expose_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        // Allocate additional capacity to the vector if no empty slots are available.
        if self.0.capacity() == self.0.len() {
            self.0.reserve((self.0.capacity() * 2) + 1);
        }

        let pointer = self.0.as_mut_ptr();
        let available_capacity = self.0.capacity() - self.0.len();

        unsafe {
            // Return a mutable slice which represents available (unused) slots.
            Ok(maybe_uninit_slice_mut(slice::from_raw_parts_mut(
                // The pointer offset.
                pointer.add(self.0.len()),
                // The length (number of slots).
                available_capacity,
            )))
        }
    }

    unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        // Update the length of the vector based on the amount of items consumed.
        self.0.set_len(self.0.len() + amount);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::sync::*;

    // The debug output hides the internals of using semantically transparent wrappers.
    #[test]
    fn debug_output_hides_transparent_wrappers() {
        let consumer: IntoVec<u8> = IntoVec::new();
        assert_eq!(format!("{:?}", consumer), "IntoVec([])");
    }

    #[test]
    fn converts_into_vec() {
        let mut into_vec = IntoVec::new();
        let _ = into_vec.bulk_consume(b"ufotofu");
        let _ = into_vec.close(());

        let vec = into_vec.into_vec();
        assert_eq!(vec.len(), 7);
    }

    // Panic conditions:
    //
    // - `consume()` must not be called after `close()` or error
    // - `close()` must not be called after `close()` or error
    // - `flush()` must not be called after `close()` or error
    // - `consumer_slots()` must not be called after `close()` or error
    // - `did_consume()` must not be called after `close()` or error
    // - `bulk_consume()` must not be called after `close()` or error
    // - `did_consume(amount)` must not be called with `amount` greater than available slots

    // In each of the following tests, the final function call should panic.

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_consume_after_close() {
        let mut into_vec = IntoVec::new();
        let _ = into_vec.close(());
        let _ = into_vec.consume(7);
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_close_after_close() {
        // Type annotations are required because we never provide a `T`.
        let mut into_vec: IntoVec<u8> = IntoVec::new();
        let _ = into_vec.close(());
        let _ = into_vec.close(());
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_flush_after_close() {
        let mut into_vec: IntoVec<u8> = IntoVec::new();
        let _ = into_vec.close(());
        let _ = into_vec.flush();
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_consumer_slots_after_close() {
        let mut into_vec: IntoVec<u8> = IntoVec::new();
        let _ = into_vec.close(());
        let _ = into_vec.expose_slots();
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_did_consume_after_close() {
        let mut into_vec: IntoVec<u8> = IntoVec::new();
        let _ = into_vec.close(());

        unsafe {
            let _ = into_vec.consume_slots(7);
        }
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_bulk_consume_after_close() {
        let mut into_vec = IntoVec::new();
        let _ = into_vec.close(());
        let _ = into_vec.bulk_consume(b"ufo");
    }

    #[test]
    #[should_panic(
        expected = "may not call `consume_slots` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_consume_with_amount_greater_than_available_slots() {
        let mut into_vec: IntoVec<u8> = IntoVec::new();

        unsafe {
            let _ = into_vec.consume_slots(21);
        }
    }
}
