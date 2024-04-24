use core::convert::AsRef;
use core::mem::MaybeUninit;

use wrapper::Wrapper;

use crate::maybe_uninit_slice_mut;
use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

/// Consumes data into a mutable slice.
pub struct Cursor<'a, T>(CursorInner<'a, T>);

/// Creates a consumer which places consumed data into the given slice.
impl<'a, T> Cursor<'a, T> {
    pub fn new(slice: &'a mut [T]) -> Cursor<'a, T> {
        Cursor(CursorInner(slice, 0))
    }
}

impl<'a, T> Wrapper<&'a [T]> for Cursor<'a, T> {
    fn into_inner(self) -> &'a [T] {
        self.0.into_inner()
    }
}

impl<'a, T> AsRef<[T]> for Cursor<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.0.as_ref()
    }
}

impl<'a, T> AsMut<[T]> for Cursor<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.0.as_mut()
    }
}

impl<'a, T> Consumer for Cursor<'a, T> {
    /// The type of the items to be consumed.
    type Item = T;
    /// The value signifying the end of the consumed sequence.
    type Final = ();
    /// The value emitted when the consumer is full and a subsequent
    /// call is made to `consume()` or `consumer_slots()`.
    type Error = ();

    fn consume(&mut self, item: T) -> Result<Self::Final, Self::Error> {
        self.0.consume(item)
    }

    fn close(&mut self, _final: Self::Final) -> Result<Self::Final, Self::Error> {
        self.0.close(_final)
    }
}

impl<'a, T: Copy> BufferedConsumer for Cursor<'a, T> {
    fn flush(&mut self) -> Result<Self::Final, Self::Error> {
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
pub struct CursorInner<'a, T>(&'a mut [T], usize);

impl<'a, T> Wrapper<&'a mut [T]> for CursorInner<'a, T> {
    fn into_inner(self) -> &'a mut [T] {
        self.0
    }
}

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

impl<'a, T> Consumer for CursorInner<'a, T> {
    /// The type of the items to be consumed.
    type Item = T;
    /// The value signifying the end of the consumed sequence.
    type Final = ();
    /// The value emitted when the consumer is full and a subsequent
    /// call is made to `consume()` or `consumer_slots()`.
    type Error = ();

    fn consume(&mut self, item: T) -> Result<Self::Final, Self::Error> {
        // The inner cursor is completely full.
        if self.0.len() == self.1 {
            Err(())
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
            Err(())
        } else {
            Ok(maybe_uninit_slice_mut(&mut self.0[self.1..]))
        }
    }

    unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.1 += amount;

        Ok(())
    }
}
