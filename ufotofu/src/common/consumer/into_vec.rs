use core::fmt::Debug;
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

use crate::common::consumer::Invariant;
use crate::maybe_uninit_slice_mut;
use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

/// Collects data and can at any point be converted into a `Vec<T>`.
pub struct IntoVec_<T, A: Allocator = Global>(Invariant<IntoVec<T, A>>);

invarianted_impl_debug!(IntoVec_<T: Debug, A: Allocator + Debug>);

impl<T> Default for IntoVec_<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntoVec_<T> {
    /// Create a new consumer that collects data into a Vec.
    pub fn new() -> IntoVec_<T> {
        let invariant = Invariant::new(IntoVec(Vec::new()));

        IntoVec_(invariant)
    }

    /// Convert `self` into the vector of all consumed items.
    pub fn into_vec(self) -> Vec<T> {
        let inner = self.0.into_inner();
        inner.into_inner()
    }

    /// Return the remaining capacity of the vector, i.e., how many more items it can consume without reallocating.
    pub fn remaining_capacity(&self) -> usize {
        self.0.as_ref().remaining_capacity()
    }

    /// Allocate capacity for at least `additional` more items. See [`std::vec::Vec::reserve`].
    pub fn reserve(&mut self, additional: usize) {
        self.0.as_mut().reserve(additional)
    }
}

impl<T, A: Allocator> IntoVec_<T, A> {
    pub fn new_in(alloc: A) -> IntoVec_<T, A> {
        let invariant = Invariant::new(IntoVec(Vec::new_in(alloc)));

        IntoVec_(invariant)
    }
}

invarianted_impl_as_ref!(IntoVec_<T, A: Allocator>; Vec<T, A>);
invarianted_impl_as_mut!(IntoVec_<T, A: Allocator>; Vec<T, A>);
invarianted_impl_wrapper!(IntoVec_<T, A: Allocator>; Vec<T, A>);

invarianted_impl_consumer_sync_and_local_nb!(IntoVec_<T, A: Allocator> Item T; Final (); Error !);
invarianted_impl_buffered_consumer_sync_and_local_nb!(IntoVec_<T, A: Allocator>);
invarianted_impl_bulk_consumer_sync_and_local_nb!(IntoVec_<T: Copy, A: Allocator>);

#[derive(Debug)]
struct IntoVec<T, A: Allocator = Global>(Vec<T, A>);

impl<T, A: Allocator> IntoVec<T, A> {
    fn remaining_capacity(&self) -> usize {
        self.0.capacity() - self.0.len()
    }

    fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional)
    }
}

impl<T, A: Allocator> AsRef<Vec<T, A>> for IntoVec<T, A> {
    fn as_ref(&self) -> &Vec<T, A> {
        &self.0
    }
}

impl<T, A: Allocator> AsMut<Vec<T, A>> for IntoVec<T, A> {
    fn as_mut(&mut self) -> &mut Vec<T, A> {
        &mut self.0
    }
}

impl<T, A: Allocator> Wrapper<Vec<T, A>> for IntoVec<T, A> {
    fn into_inner(self) -> Vec<T, A> {
        self.0
    }
}

impl<T, A: Allocator> Consumer for IntoVec<T, A> {
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

impl<T, A: Allocator> BufferedConsumer for IntoVec<T, A> {
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T: Copy, A: Allocator> BulkConsumer for IntoVec<T, A> {
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

sync_consumer_as_local_nb!(IntoVec<T, A: Allocator>);
sync_buffered_consumer_as_local_nb!(IntoVec<T, A: Allocator>);
sync_bulk_consumer_as_local_nb!(IntoVec<T: Copy, A: Allocator>);

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::sync::*;

    use std::format;

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
