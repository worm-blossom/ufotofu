use core::convert::{AsMut, AsRef};
use core::mem::MaybeUninit;

use wrapper::Wrapper;

use crate::maybe_uninit_slice_mut;
use crate::sync::consumer::Invariant;
use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

#[derive(Debug)]
pub struct CursorFullError;

impl From<!> for CursorFullError {
    fn from(never: !) -> CursorFullError {
        match never {}
    }
}

/// Consumes data into a mutable slice.
#[derive(Debug)]
pub struct Cursor<'a, T>(Invariant<CursorInner<'a, T>>);

/// Creates a consumer which places consumed data into the given slice.
impl<'a, T> Cursor<'a, T> {
    pub fn new(slice: &mut [T]) -> Cursor<'_, T> {
        // Wrap the inner cursor in the invariant type.
        let invariant = Invariant::new(CursorInner(slice, 0));

        Cursor(invariant)
    }
}

impl<'a, T> AsRef<[T]> for Cursor<'a, T> {
    fn as_ref(&self) -> &[T] {
        let inner = self.0.as_ref();
        inner.as_ref()
    }
}

impl<'a, T> AsMut<[T]> for Cursor<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        let inner = self.0.as_mut();
        inner.as_mut()
    }
}

impl<'a, T> Wrapper<&'a [T]> for Cursor<'a, T> {
    fn into_inner(self) -> &'a [T] {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<'a, T: Copy> Consumer for Cursor<'a, T> {
    /// The type of the items to be consumed.
    type Item = T;
    /// The value signifying the end of the consumed sequence.
    type Final = ();
    /// The value emitted when the consumer is full and a subsequent
    /// call is made to `consume()` or `consumer_slots()`.
    type Error = CursorFullError;

    fn consume(&mut self, item: T) -> Result<(), Self::Error> {
        self.0.consume(item)
    }

    fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        self.0.close(final_val)
    }
}

impl<'a, T: Copy> BufferedConsumer for Cursor<'a, T> {
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush()
    }
}

impl<'a, T: Copy> BulkConsumer for Cursor<'a, T> {
    fn consumer_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        self.0.consumer_slots()
    }

    unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.did_consume(amount)
    }
}

// A tuple of slice and counter (amount of items).
#[derive(Debug)]
pub struct CursorInner<'a, T>(&'a mut [T], usize);

impl<'a, T> AsRef<[T]> for CursorInner<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.0
    }
}

impl<'a, T> AsMut<[T]> for CursorInner<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.0
    }
}

impl<'a, T> Wrapper<&'a mut [T]> for CursorInner<'a, T> {
    fn into_inner(self) -> &'a mut [T] {
        self.0
    }
}

impl<'a, T> Consumer for CursorInner<'a, T> {
    /// The type of the items to be consumed.
    type Item = T;
    /// The value signifying the end of the consumed sequence.
    type Final = ();
    /// The value emitted when the consumer is full and a subsequent
    /// call is made to `consume()` or `consumer_slots()`.
    type Error = CursorFullError;

    fn consume(&mut self, item: T) -> Result<Self::Final, Self::Error> {
        // The inner cursor is completely full.
        if self.0.len() == self.1 {
            Err(CursorFullError)
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

impl<'a, T: Copy> BufferedConsumer for CursorInner<'a, T> {
    fn flush(&mut self) -> Result<Self::Final, Self::Error> {
        Ok(())
    }
}

impl<'a, T: Copy> BulkConsumer for CursorInner<'a, T> {
    fn consumer_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        if self.0.len() == self.1 {
            Err(CursorFullError)
        } else {
            Ok(maybe_uninit_slice_mut(&mut self.0[self.1..]))
        }
    }

    unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.1 += amount;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let mut cursor = Cursor::new(&mut buf);
        let _ = cursor.close(());
        let _ = cursor.consume(7);
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_close_after_close() {
        let mut buf = [0; 1];

        let mut cursor = Cursor::new(&mut buf);
        let _ = cursor.close(());
        let _ = cursor.close(());
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_flush_after_close() {
        let mut buf = [0; 1];

        let mut cursor = Cursor::new(&mut buf);
        let _ = cursor.close(());
        let _ = cursor.flush();
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_consumer_slots_after_close() {
        let mut buf = [0; 1];

        let mut cursor = Cursor::new(&mut buf);
        let _ = cursor.close(());
        let _ = cursor.consumer_slots();
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_did_consume_after_close() {
        let mut buf = [0; 8];

        let mut cursor = Cursor::new(&mut buf);
        let _ = cursor.close(());

        unsafe {
            let _ = cursor.did_consume(7);
        }
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_bulk_consume_after_close() {
        let mut buf = [0; 8];

        let mut cursor = Cursor::new(&mut buf);
        let _ = cursor.close(());
        let _ = cursor.bulk_consume(b"ufo");
    }

    #[test]
    #[should_panic(
        expected = "may not call `did_consume` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_consume_with_amount_greater_than_available_slots() {
        let mut buf = [0; 8];

        let mut cursor = Cursor::new(&mut buf);

        unsafe {
            let _ = cursor.did_consume(21);
        }
    }
}
