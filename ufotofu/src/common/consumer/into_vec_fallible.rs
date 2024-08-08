use core::fmt::Debug;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{
    alloc::{Allocator, Global},
    collections::TryReserveError,
    vec::Vec,
};
#[cfg(feature = "std")]
use std::{collections::TryReserveError, vec::Vec};

use wrapper::Wrapper;

use crate::common::consumer::Invariant;
use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

/// Collects data and can at any point be converted into a `Vec<T>`. Unlike [`IntoVec`](crate::sync::consumer::IntoVec), reports an error instead of panicking when an internal memory allocation fails.
pub struct IntoVecFallible_<T>(Invariant<IntoVecFallible<T>>);

invarianted_impl_debug!(IntoVecFallible_<T: Debug>);

impl<T> Default for IntoVecFallible_<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntoVecFallible_<T> {
    pub fn new() -> IntoVecFallible_<T> {
        let invariant = Invariant::new(IntoVecFallible {
            v: Vec::new(),
            consumed: 0,
        });

        IntoVecFallible_(invariant)
    }

    pub fn into_vec(self) -> Vec<T> {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<T: Default> IntoVecFallible_<T> {
    pub(crate) fn make_space_if_needed(&mut self) -> Result<(), TryReserveError> {
        self.0.as_mut().make_space_if_needed()
    }

    pub(crate) fn remaining_slots(&self) -> usize {
        self.0.as_ref().remaining_slots()
    }
}

invarianted_impl_as_ref!(IntoVecFallible_<T>; Vec<T>);
invarianted_impl_as_mut!(IntoVecFallible_<T>; Vec<T>);
invarianted_impl_wrapper!(IntoVecFallible_<T>; Vec<T>);

invarianted_impl_consumer_sync_and_local_nb!(IntoVecFallible_<T: Default> Item T; Final (); Error TryReserveError);
invarianted_impl_buffered_consumer_sync_and_local_nb!(IntoVecFallible_<T: Default>);
invarianted_impl_bulk_consumer_sync_and_local_nb!(IntoVecFallible_<T: Copy + Default>);

#[derive(Debug)]
struct IntoVecFallible<T> {
    v: Vec<T>,
    consumed: usize,
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
        let IntoVecFallible { mut v, consumed } = self;
        v.truncate(consumed);
        v
    }
}

impl<T: Default> IntoVecFallible<T> {
    fn make_space_if_needed(&mut self) -> Result<(), TryReserveError> {
        // Allocate additional capacity to the vector if no empty slots are available.
        if self.consumed == self.v.len() {
            // Will return an error if capacity overflows or the allocator reports a failure.
            self.v.try_reserve(self.consumed + 1)?;
            self.v.resize_with(self.consumed * 2 + 1, Default::default); // Does not allocate because we reserved before.
        }

        Ok(())
    }

    fn remaining_slots(&self) -> usize {
        self.v.len() - self.consumed
    }
}

impl<T: Default> Consumer for IntoVecFallible<T> {
    type Item = T;
    type Final = ();
    type Error = TryReserveError;

    fn consume(&mut self, item: T) -> Result<Self::Final, Self::Error> {
        // Allocate additional capacity to the vector if no empty slots are available.
        self.make_space_if_needed()?;

        self.v[self.consumed] = item;
        self.consumed += 1;

        Ok(())
    }

    fn close(&mut self, _fin: Self::Final) -> Result<Self::Final, Self::Error> {
        Ok(())
    }
}

impl<T: Default> BufferedConsumer for IntoVecFallible<T> {
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T: Default + Copy> BulkConsumer for IntoVecFallible<T> {
    fn expose_slots(&mut self) -> Result<&mut [Self::Item], Self::Error> {
        // Allocate additional capacity to the vector if no empty slots are available.
        self.make_space_if_needed()?;

        Ok(&mut self.v[self.consumed..])
    }

    fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.consumed += amount;

        Ok(())
    }
}

sync_consumer_as_local_nb!(IntoVecFallible<T: Default>);
sync_buffered_consumer_as_local_nb!(IntoVecFallible<T: Default>);
sync_bulk_consumer_as_local_nb!(IntoVecFallible<T: Copy + Default>);

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::sync::*;

    #[test]
    fn converts_into_vec() {
        let mut into_vec = IntoVecFallible::new();
        let _ = into_vec.bulk_consume_full_slice(b"ufotofu");
        let _ = into_vec.close(());

        let v = into_vec.into_vec();
        assert_eq!(v, std::vec![117, 102, 111, 116, 111, 102, 117]);
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

        let _ = into_vec.consume_slots(7);
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

        let _ = into_vec.consume_slots(21);
    }
}
