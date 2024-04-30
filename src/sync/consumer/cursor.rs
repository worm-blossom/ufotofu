use core::convert::AsRef;
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
pub struct Cursor<I>(Invariant<I>);

/// Creates a consumer which places consumed data into the given slice.
impl<I> Cursor<I> {
    pub fn new<T>(slice: &mut [T]) -> Cursor<CursorInner<'_, T>> {
        // Wrap the inner cursor in the invariant type.
        let invariant = Invariant::new(CursorInner(slice, 0));

        Cursor(invariant)
    }
}

impl<I, T> AsRef<[T]> for Cursor<I>
where
    I: AsRef<[T]>,
{
    fn as_ref(&self) -> &[T] {
        let inner = self.0.as_ref();
        inner.as_ref()
    }
}

impl<I, T> AsMut<[T]> for Cursor<I>
where
    I: AsMut<[T]>,
{
    fn as_mut(&mut self) -> &mut [T] {
        let inner = self.0.as_mut();
        inner.as_mut()
    }
}

impl<'a, I, T> Wrapper<&'a [T]> for Cursor<I>
where
    I: Wrapper<&'a [T]>,
{
    fn into_inner(self) -> &'a [T] {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<I, T, F, E> Consumer for Cursor<I>
where
    I: BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    /// The type of the items to be consumed.
    type Item = T;
    /// The value signifying the end of the consumed sequence.
    type Final = F;
    /// The value emitted when the consumer is full and a subsequent
    /// call is made to `consume()` or `consumer_slots()`.
    type Error = E;

    fn consume(&mut self, item: T) -> Result<(), Self::Error> {
        self.0.consume(item)
    }

    fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        self.0.close(final_val)
    }
}

impl<I, T, F, E> BufferedConsumer for Cursor<I>
where
    I: BufferedConsumer<Item = T, Final = F, Error = E>
        + BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush()
    }
}

impl<I, T, F, E> BulkConsumer for Cursor<I>
where
    I: BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn consumer_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        self.0.consumer_slots()
    }

    unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.did_consume(amount)
    }
}

// A tuple of slice and counter (amount of items).
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
