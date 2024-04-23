use core::convert::AsRef;

use either::Either;
use wrapper::Wrapper;

use crate::sync::{BufferedProducer, BulkProducer, Producer};

/// Produces data from a slice.
pub struct Cursor<'a, T>(CursorInner<'a, T>);

/// Creates a producer which produces the data in the given slice.
impl<'a, T> Cursor<'a, T> {
    pub fn new(slice: &'a [T]) -> Cursor<'a, T> {
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

impl<'a, T: Clone> Producer for Cursor<'a, T> {
    /// The type of the items to be produced.
    type Item = T;
    /// The final value emitted once the end of the slice has been reached.
    type Final = ();
    // This implementation of Producer cannot error.
    type Error = !;

    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce()
    }
}

impl<'a, T: Copy> BufferedProducer for Cursor<'a, T> {
    fn slurp(&mut self) -> Result<(), Self::Error> {
        self.0.slurp()
    }
}

impl<'a, T: Copy> BulkProducer for Cursor<'a, T> {
    fn producer_slots(&self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        self.0.producer_slots()
    }

    fn did_produce(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.did_produce(amount)
    }
}

pub struct CursorInner<'a, T>(&'a [T], usize);

impl<'a, T> Wrapper<&'a [T]> for CursorInner<'a, T> {
    fn into_inner(self) -> &'a [T] {
        self.0
    }
}

impl<'a, T> AsRef<[T]> for CursorInner<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.0
    }
}

impl<'a, T: Clone> Producer for CursorInner<'a, T> {
    /// The type of the items to be produced.
    type Item = T;
    /// The final value emitted once the end of the slice has been reached.
    type Final = ();
    // This implementation of Producer cannot error.
    type Error = !;

    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        if self.0.len() == self.1 {
            Ok(Either::Right(()))
        } else {
            // Clone the item from the slice at the given index.
            let item = self.0[self.1].clone();
            // Increment the item counter.
            self.1 += 1;

            Ok(Either::Left(item))
        }
    }
}

impl<'a, T: Copy> BufferedProducer for CursorInner<'a, T> {
    fn slurp(&mut self) -> Result<(), Self::Error> {
        // There are no effects to perform so we simply return.
        Ok(())
    }
}

impl<'a, T: Copy> BulkProducer for CursorInner<'a, T> {
    fn producer_slots(&self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        let slice = &self.0[self.1..];
        if slice.is_empty() {
            Ok(Either::Right(()))
        } else {
            Ok(Either::Left(slice))
        }
    }

    fn did_produce(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.1 += amount;

        Ok(())
    }
}
