use core::mem::MaybeUninit;
use core::slice;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{
    alloc::{Allocator, Global},
    collections::TryReserveError,
    vec::Vec,
};
#[cfg(feature = "std")]
use std::{
    alloc::{Allocator, Global},
    collections::TryReserveError,
    vec::Vec,
};

use thiserror::Error;
use wrapper::Wrapper;

use crate::common::consumer::Invariant;
use crate::local_nb::{
    BufferedConsumer as BufferedConsumerLocalNb, BulkConsumer as BulkConsumerLocalNb,
    Consumer as ConsumerLocalNb,
};
use crate::maybe_uninit_slice_mut;
use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error(transparent)]
/// Error to indicate that consuming data into a `Vec` failed because allocating more memory for the `Vec` failed.
pub struct IntoVecError(#[from] pub TryReserveError);

/// Collects data and can at any point be converted into a `Vec<T>`. Unlike [`IntoVec`](crate::sync::consumer::IntoVec), reports an error instead of panicking when an internal memory allocation fails.
pub struct IntoVecFallible_<T, A: Allocator = Global>(Invariant<IntoVecFallible<T, A>>);

impl<T: core::fmt::Debug, A: Allocator + core::fmt::Debug> core::fmt::Debug
    for IntoVecFallible_<T, A>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> Default for IntoVecFallible_<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntoVecFallible_<T> {
    pub fn new() -> IntoVecFallible_<T> {
        let invariant = Invariant::new(IntoVecFallible { v: Vec::new() });

        IntoVecFallible_(invariant)
    }

    pub fn into_vec(self) -> Vec<T> {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<T, A: Allocator> IntoVecFallible_<T, A> {
    pub fn new_in(alloc: A) -> IntoVecFallible_<T, A> {
        let invariant = Invariant::new(IntoVecFallible {
            v: Vec::new_in(alloc),
        });

        IntoVecFallible_(invariant)
    }
}

impl<T> AsRef<Vec<T>> for IntoVecFallible_<T> {
    fn as_ref(&self) -> &Vec<T> {
        let inner = self.0.as_ref();
        inner.as_ref()
    }
}

impl<T> AsMut<Vec<T>> for IntoVecFallible_<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        let inner = self.0.as_mut();
        inner.as_mut()
    }
}

impl<T> Wrapper<Vec<T>> for IntoVecFallible_<T> {
    fn into_inner(self) -> Vec<T> {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<T> Consumer for IntoVecFallible_<T> {
    type Item = T;
    type Final = ();
    type Error = IntoVecError;

    fn consume(&mut self, item: T) -> Result<(), Self::Error> {
        Consumer::consume(&mut self.0, item)
    }

    fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error> {
        Consumer::close(&mut self.0, fin)
    }
}

impl<T> BufferedConsumer for IntoVecFallible_<T> {
    fn flush(&mut self) -> Result<(), Self::Error> {
        BufferedConsumer::flush(&mut self.0)
    }
}

impl<T: Copy> BulkConsumer for IntoVecFallible_<T> {
    fn expose_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        BulkConsumer::expose_slots(&mut self.0)
    }

    unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        BulkConsumer::consume_slots(&mut self.0, amount)
    }
}

impl<T> ConsumerLocalNb for IntoVecFallible_<T> {
    type Item = T;
    type Final = ();
    type Error = IntoVecError;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        ConsumerLocalNb::consume(&mut self.0, item).await
    }

    async fn close(&mut self, f: Self::Final) -> Result<(), Self::Error> {
        ConsumerLocalNb::close(&mut self.0, f).await
    }
}

impl<T> BufferedConsumerLocalNb for IntoVecFallible_<T> {
    async fn flush(&mut self) -> Result<Self::Final, Self::Error> {
        BufferedConsumerLocalNb::flush(&mut self.0).await
    }
}

impl<T: Copy> BulkConsumerLocalNb for IntoVecFallible_<T> {
    async fn expose_slots<'a>(
        &'a mut self,
    ) -> Result<&'a mut [MaybeUninit<Self::Item>], Self::Error>
    where
        Self::Item: 'a,
    {
        BulkConsumerLocalNb::expose_slots(&mut self.0).await
    }

    async unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        BulkConsumerLocalNb::consume_slots(&mut self.0, amount).await
    }
}

#[derive(Debug)]
struct IntoVecFallible<T, A: Allocator = Global> {
    v: Vec<T, A>,
}

impl<T> AsRef<Vec<T>> for IntoVecFallible<T> {
    fn as_ref(&self) -> &Vec<T> {
        &self.v
    }
}

impl<T> AsMut<Vec<T>> for IntoVecFallible<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        &mut self.v
    }
}

impl<T> Wrapper<Vec<T>> for IntoVecFallible<T> {
    fn into_inner(self) -> Vec<T> {
        self.v
    }
}

impl<T> Consumer for IntoVecFallible<T> {
    type Item = T;
    type Final = ();
    type Error = IntoVecError;

    fn consume(&mut self, item: T) -> Result<Self::Final, Self::Error> {
        if let Err(value) = self.v.push_within_capacity(item) {
            self.v.try_reserve(1)?;
            // This cannot fail; the previous line either returned or added
            // at least 1 free slot.
            let _ = self.v.push_within_capacity(value);
        }

        Ok(())
    }

    fn close(&mut self, _final: Self::Final) -> Result<Self::Final, Self::Error> {
        Ok(())
    }
}

impl<T> BufferedConsumer for IntoVecFallible<T> {
    fn flush(&mut self) -> Result<Self::Final, Self::Error> {
        Ok(())
    }
}

impl<T: Copy> BulkConsumer for IntoVecFallible<T> {
    fn expose_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        // Allocate additional capacity to the vector if no empty slots are available.
        if self.v.capacity() == self.v.len() {
            // Will return an error if capacity overflows or the allocator reports a failure.
            self.v.try_reserve((self.v.capacity() * 2) + 1)?;
        }

        let pointer = self.v.as_mut_ptr();
        let available_capacity = self.v.capacity() - self.v.len();

        unsafe {
            // Return a mutable slice which represents available (unused) slots.
            Ok(maybe_uninit_slice_mut(slice::from_raw_parts_mut(
                // The pointer offset.
                pointer.add(self.v.len()),
                // The length (number of slots).
                available_capacity,
            )))
        }
    }

    unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        // Update the length of the vector based on the amount of items consumed.
        self.v.set_len(self.v.len() + amount);

        Ok(())
    }
}

impl<T> ConsumerLocalNb for IntoVecFallible<T> {
    type Item = T;
    type Final = ();
    type Error = IntoVecError;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        Consumer::consume(self, item)
    }

    async fn close(&mut self, f: Self::Final) -> Result<(), Self::Error> {
        Consumer::close(self, f)
    }
}

impl<T> BufferedConsumerLocalNb for IntoVecFallible<T> {
    async fn flush(&mut self) -> Result<Self::Final, Self::Error> {
        BufferedConsumer::flush(self)
    }
}

impl<T: Copy> BulkConsumerLocalNb for IntoVecFallible<T> {
    async fn expose_slots<'a>(
        &'a mut self,
    ) -> Result<&'a mut [MaybeUninit<Self::Item>], Self::Error>
    where
        Self::Item: 'a,
    {
        BulkConsumer::expose_slots(self)
    }

    async unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        BulkConsumer::consume_slots(self, amount)
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::sync::*;

    #[test]
    fn converts_into_vec() {
        let mut into_vec = IntoVecFallible::new();
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
        let mut into_vec = IntoVecFallible::new();
        let _ = into_vec.close(());
        let _ = into_vec.consume(7);
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_close_after_close() {
        // Type annotations are required because we never provide a `T`.
        let mut into_vec: IntoVecFallible<u8> = IntoVecFallible::new();
        let _ = into_vec.close(());
        let _ = into_vec.close(());
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_flush_after_close() {
        let mut into_vec: IntoVecFallible<u8> = IntoVecFallible::new();
        let _ = into_vec.close(());
        let _ = into_vec.flush();
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_consumer_slots_after_close() {
        let mut into_vec: IntoVecFallible<u8> = IntoVecFallible::new();
        let _ = into_vec.close(());
        let _ = into_vec.expose_slots();
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_did_consume_after_close() {
        let mut into_vec: IntoVecFallible<u8> = IntoVecFallible::new();
        let _ = into_vec.close(());

        unsafe {
            let _ = into_vec.consume_slots(7);
        }
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_bulk_consume_after_close() {
        let mut into_vec = IntoVecFallible::new();
        let _ = into_vec.close(());
        let _ = into_vec.bulk_consume(b"ufo");
    }

    #[test]
    #[should_panic(
        expected = "may not call `consume_slots` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_consume_with_amount_greater_than_available_slots() {
        let mut into_vec: IntoVecFallible<u8> = IntoVecFallible::new();

        unsafe {
            let _ = into_vec.consume_slots(21);
        }
    }
}
