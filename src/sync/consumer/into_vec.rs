use core::mem::MaybeUninit;
use core::slice;

use std::alloc::{Allocator, Global};
use std::vec::Vec;

use wrapper::Wrapper;

use crate::maybe_uninit_slice_mut;
use crate::sync::consumer::Invariant;
use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

pub struct IntoVec<T, A: Allocator = Global>(Invariant<IntoVecInner<T, A>>);

impl<T> Default for IntoVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntoVec<T> {
    pub fn new() -> IntoVec<T> {
        let invariant = Invariant::new(IntoVecInner { v: Vec::new() });

        IntoVec(invariant)
    }

    pub fn into_vec(self) -> Vec<T> {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<T, A> IntoVec<T, A>
where
    A: Allocator,
{
    pub fn new_in(alloc: A) -> IntoVec<T, A> {
        let invariant = Invariant::new(IntoVecInner {
            v: Vec::new_in(alloc),
        });

        IntoVec(invariant)
    }
}

impl<T> AsRef<Vec<T>> for IntoVec<T> {
    fn as_ref(&self) -> &Vec<T> {
        let inner = self.0.as_ref();
        inner.as_ref()
    }
}

impl<T> AsMut<Vec<T>> for IntoVec<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        let inner = self.0.as_mut();
        inner.as_mut()
    }
}

impl<T> Wrapper<Vec<T>> for IntoVec<T> {
    fn into_inner(self) -> Vec<T> {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<T> Consumer for IntoVec<T> {
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

impl<T> BufferedConsumer for IntoVec<T> {
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush()
    }
}

impl<T: Copy> BulkConsumer for IntoVec<T> {
    fn consumer_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        self.0.consumer_slots()
    }

    unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.did_consume(amount)
    }
}

/// Collects data and can at any point be converted into a `Vec<T>`.
pub struct IntoVecInner<T, A: Allocator = Global> {
    v: Vec<T, A>,
}

impl<T> AsRef<Vec<T>> for IntoVecInner<T> {
    fn as_ref(&self) -> &Vec<T> {
        &self.v
    }
}

impl<T> AsMut<Vec<T>> for IntoVecInner<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        &mut self.v
    }
}

impl<T> Wrapper<Vec<T>> for IntoVecInner<T> {
    fn into_inner(self) -> Vec<T> {
        self.v
    }
}

impl<T> Consumer for IntoVecInner<T> {
    type Item = T;
    type Final = ();
    type Error = !;

    fn consume(&mut self, item: T) -> Result<Self::Final, Self::Error> {
        self.v.push(item);

        Ok(())
    }

    fn close(&mut self, _final: Self::Final) -> Result<Self::Final, Self::Error> {
        Ok(())
    }
}

impl<T> BufferedConsumer for IntoVecInner<T> {
    fn flush(&mut self) -> Result<Self::Final, Self::Error> {
        Ok(())
    }
}

impl<T: Copy> BulkConsumer for IntoVecInner<T> {
    fn consumer_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        // Allocate additional capacity to the vector if no empty slots are available.
        if self.v.capacity() == self.v.len() {
            self.v.reserve((self.v.capacity() * 2) + 1);
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

    unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        // Update the length of the vector based on the amount of items consumed.
        self.v.set_len(self.v.len() + amount);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let _ = into_vec.consumer_slots();
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_did_consume_after_close() {
        let mut into_vec: IntoVec<u8> = IntoVec::new();
        let _ = into_vec.close(());

        unsafe {
            let _ = into_vec.did_consume(7);
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
        expected = "may not call `did_consume` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_consume_with_amount_greater_than_available_slots() {
        let mut into_vec: IntoVec<u8> = IntoVec::new();

        unsafe {
            let _ = into_vec.did_consume(21);
        }
    }
}
