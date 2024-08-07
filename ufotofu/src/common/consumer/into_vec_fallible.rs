use core::fmt::Debug;

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

use wrapper::Wrapper;

use crate::common::consumer::Invariant;
use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

/// Collects data and can at any point be converted into a `Vec<T>`. Unlike [`IntoVec`](crate::sync::consumer::IntoVec), reports an error instead of panicking when an internal memory allocation fails.
pub struct IntoVecFallible_<T, A: Allocator = Global>(Invariant<IntoVecFallible<T, A>>);

invarianted_impl_debug!(IntoVecFallible_<T: Debug, A: Allocator + Debug>);

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

impl<T: Default, A: Allocator> IntoVecFallible_<T, A> {
    pub(crate) fn make_space_if_needed(&mut self) -> Result<(), TryReserveError> {
        self.0.as_mut().make_space_if_needed()
    }

    pub(crate) fn remaining_slots(&self) -> usize {
        self.0.as_ref().remaining_slots()
    }
}

impl<T, A: Allocator> IntoVecFallible_<T, A> {
    pub fn new_in(alloc: A) -> IntoVecFallible_<T, A> {
        let invariant = Invariant::new(IntoVecFallible {
            v: Vec::new_in(alloc),
            consumed: 0,
        });

        IntoVecFallible_(invariant)
    }
}

invarianted_impl_as_ref!(IntoVecFallible_<T, A: Allocator>; Vec<T, A>);
invarianted_impl_as_mut!(IntoVecFallible_<T, A: Allocator>; Vec<T, A>);
invarianted_impl_wrapper!(IntoVecFallible_<T, A: Allocator>; Vec<T, A>);

invarianted_impl_consumer_sync_and_local_nb!(IntoVecFallible_<T: Default, A: Allocator> Item T; Final (); Error TryReserveError);
invarianted_impl_buffered_consumer_sync_and_local_nb!(IntoVecFallible_<T: Default, A: Allocator>);
invarianted_impl_bulk_consumer_sync_and_local_nb!(IntoVecFallible_<T: Copy + Default, A: Allocator>);

#[derive(Debug)]
struct IntoVecFallible<T, A: Allocator = Global> {
    v: Vec<T, A>,
    consumed: usize,
}

impl<T, A: Allocator> AsRef<Vec<T, A>> for IntoVecFallible<T, A> {
    fn as_ref(&self) -> &Vec<T, A> {
        &self.v
    }
}

impl<T, A: Allocator> AsMut<Vec<T, A>> for IntoVecFallible<T, A> {
    fn as_mut(&mut self) -> &mut Vec<T, A> {
        &mut self.v
    }
}

impl<T, A: Allocator> Wrapper<Vec<T, A>> for IntoVecFallible<T, A> {
    fn into_inner(self) -> Vec<T, A> {
        self.v
    }
}

impl<T: Default, A: Allocator> IntoVecFallible<T, A> {
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

impl<T: Default, A: Allocator> Consumer for IntoVecFallible<T, A> {
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

impl<T: Default, A: Allocator> BufferedConsumer for IntoVecFallible<T, A> {
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T: Default + Copy, A: Allocator> BulkConsumer for IntoVecFallible<T, A> {
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

sync_consumer_as_local_nb!(IntoVecFallible<T: Default, A: Allocator>);
sync_buffered_consumer_as_local_nb!(IntoVecFallible<T: Default, A: Allocator>);
sync_bulk_consumer_as_local_nb!(IntoVecFallible<T: Copy + Default, A: Allocator>);

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
