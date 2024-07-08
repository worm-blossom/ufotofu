use core::convert::{AsMut, AsRef};
use core::mem::MaybeUninit;

use thiserror::Error;
use wrapper::Wrapper;

use crate::maybe_uninit_slice_mut;
use crate::sync::consumer::Invariant;
use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[error("slice consumer is full")]
/// Error to indicate that consuming data into a slice failed because the end of the slice was reached.
pub struct SliceConsumerFullError;

/// Consumes data into a mutable slice.
pub struct SliceConsumer_<'a, T>(Invariant<SliceConsumer<'a, T>>);

impl<'a, T: core::fmt::Debug> core::fmt::Debug for SliceConsumer_<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Creates a consumer which places consumed data into the given slice.
impl<'a, T> SliceConsumer_<'a, T> {
    pub fn new(slice: &mut [T]) -> SliceConsumer_<'_, T> {
        // Wrap the inner slice consumer in the invariant type.
        let invariant = Invariant::new(SliceConsumer(slice, 0));

        SliceConsumer_(invariant)
    }

    /// Return the offset into the slice at which the next item consumed item will be written.
    pub fn get_offset(&self) -> usize {
        (self.0).as_ref().1
    }

    /// Return the subslice that has been overwritten with consumed items.
    pub fn get_overwritten_so_far(&self) -> &[T] {
        &(self.0).as_ref().0[..self.get_offset()]
    }

    /// Return the subslice of items that have not yet been overwritten with consumed items.
    pub fn get_not_yet_overwritten(&self) -> &[T] {
        &(self.0).as_ref().0[self.get_offset()..]
    }

    /// Return a mutable reference to the subslice that has been overwritten with consumed items.
    pub fn get_overwritten_so_far_mut(&mut self) -> &mut [T] {
        let offset = self.get_offset();
        &mut (self.0).as_mut().0[..offset]
    }

    /// Return a mutable reference to the subslice of items that have not yet been overwritten with consumed items.
    pub fn get_not_yet_overwritten_mut(&mut self) -> &mut [T] {
        let offset = self.get_offset();
        &mut (self.0).as_mut().0[offset..]
    }
}

impl<'a, T> AsRef<[T]> for SliceConsumer_<'a, T> {
    fn as_ref(&self) -> &[T] {
        let inner = self.0.as_ref();
        inner.as_ref()
    }
}

impl<'a, T> AsMut<[T]> for SliceConsumer_<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        let inner = self.0.as_mut();
        inner.as_mut()
    }
}

impl<'a, T> Wrapper<&'a [T]> for SliceConsumer_<'a, T> {
    fn into_inner(self) -> &'a [T] {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<'a, T> Consumer for SliceConsumer_<'a, T> {
    /// The type of the items to be consumed.
    type Item = T;
    /// The value signifying the end of the consumed sequence.
    type Final = ();
    /// The value emitted when the consumer is full and a subsequent
    /// call is made to `consume()` or `consumer_slots()`.
    type Error = SliceConsumerFullError;

    fn consume(&mut self, item: T) -> Result<(), Self::Error> {
        self.0.consume(item)
    }

    fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        self.0.close(final_val)
    }
}

impl<'a, T> BufferedConsumer for SliceConsumer_<'a, T> {
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush()
    }
}

impl<'a, T: Copy> BulkConsumer for SliceConsumer_<'a, T> {
    fn expose_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        self.0.expose_slots()
    }

    unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.consume_slots(amount)
    }
}

// A tuple of slice and counter (amount of items).
#[derive(Debug)]
struct SliceConsumer<'a, T>(&'a mut [T], usize);

impl<'a, T> AsRef<[T]> for SliceConsumer<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.0
    }
}

impl<'a, T> AsMut<[T]> for SliceConsumer<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.0
    }
}

impl<'a, T> Wrapper<&'a mut [T]> for SliceConsumer<'a, T> {
    fn into_inner(self) -> &'a mut [T] {
        self.0
    }
}

impl<'a, T> Consumer for SliceConsumer<'a, T> {
    /// The type of the items to be consumed.
    type Item = T;
    /// The value signifying the end of the consumed sequence.
    type Final = ();
    /// The value emitted when the consumer is full and a subsequent
    /// call is made to `consume()` or `consumer_slots()`.
    type Error = SliceConsumerFullError;

    fn consume(&mut self, item: T) -> Result<Self::Final, Self::Error> {
        // The inner slice consumer is completely full.
        if self.0.len() == self.1 {
            Err(SliceConsumerFullError)
        } else {
            // Copy the item to the slice at the given index.
            self.0[self.1] = item;
            // Increment the item counter.
            self.1 += 1;

            Ok(())
        }
    }

    fn close(&mut self, _: Self::Final) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a, T> BufferedConsumer for SliceConsumer<'a, T> {
    fn flush(&mut self) -> Result<Self::Final, Self::Error> {
        Ok(())
    }
}

impl<'a, T: Copy> BulkConsumer for SliceConsumer<'a, T> {
    fn expose_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        if self.0.len() == self.1 {
            Err(SliceConsumerFullError)
        } else {
            Ok(maybe_uninit_slice_mut(&mut self.0[self.1..]))
        }
    }

    unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.1 += amount;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::sync::*;

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
        let mut buf = [0; 1];

        let mut slice_consumer = SliceConsumer::new(&mut buf);
        let _ = slice_consumer.close(());
        let _ = slice_consumer.consume(7);
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_close_after_close() {
        let mut buf = [0; 1];

        let mut slice_consumer = SliceConsumer::new(&mut buf);
        let _ = slice_consumer.close(());
        let _ = slice_consumer.close(());
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_flush_after_close() {
        let mut buf = [0; 1];

        let mut slice_consumer = SliceConsumer::new(&mut buf);
        let _ = slice_consumer.close(());
        let _ = slice_consumer.flush();
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_consumer_slots_after_close() {
        let mut buf = [0; 1];

        let mut slice_consumer = SliceConsumer::new(&mut buf);
        let _ = slice_consumer.close(());
        let _ = slice_consumer.expose_slots();
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_did_consume_after_close() {
        let mut buf = [0; 8];

        let mut slice_consumer = SliceConsumer::new(&mut buf);
        let _ = slice_consumer.close(());

        unsafe {
            let _ = slice_consumer.consume_slots(7);
        }
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_bulk_consume_after_close() {
        let mut buf = [0; 8];

        let mut slice_consumer = SliceConsumer::new(&mut buf);
        let _ = slice_consumer.close(());
        let _ = slice_consumer.bulk_consume(b"ufo");
    }

    #[test]
    #[should_panic(
        expected = "may not call `consume_slots` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_consume_with_amount_greater_than_available_slots() {
        let mut buf = [0; 8];

        let mut slice_consumer = SliceConsumer::new(&mut buf);

        unsafe {
            let _ = slice_consumer.consume_slots(21);
        }
    }
}
