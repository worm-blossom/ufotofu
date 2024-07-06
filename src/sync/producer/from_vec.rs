use core::convert::AsRef;
use std::vec::Vec;

use either::Either;
use wrapper::Wrapper;

use crate::sync::producer::Invariant;
use crate::sync::{BufferedProducer, BulkProducer, Producer};

#[derive(Debug)]
/// Produces data from a slice.
pub struct FromVec<T>(Invariant<FromVecInner<T>>);

impl<T> FromVec<T> {
    /// Create a producer which produces the data in the given slice.
    pub fn new(v: Vec<T>) -> FromVec<T> {
        let invariant = Invariant::new(FromVecInner(v, 0));

        FromVec(invariant)
    }
}

impl<T> AsRef<[T]> for FromVec<T> {
    fn as_ref(&self) -> &[T] {
        let inner = self.0.as_ref();
        inner.as_ref()
    }
}

impl<T> Wrapper<Vec<T>> for FromVec<T> {
    fn into_inner(self) -> Vec<T> {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<T: Clone> Producer for FromVec<T> {
    /// The type of the items to be produced.
    type Item = T;
    /// The final value emitted once the end of the slice has been reached.
    type Final = ();
    /// The producer can never error.
    type Error = !;

    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce()
    }
}

impl<T: Copy> BufferedProducer for FromVec<T> {
    fn slurp(&mut self) -> Result<(), Self::Error> {
        self.0.slurp()
    }
}

impl<T: Copy> BulkProducer for FromVec<T> {
    fn producer_slots(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        self.0.producer_slots()
    }

    fn did_produce(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.did_produce(amount)
    }
}

#[derive(Debug)]
pub struct FromVecInner<T>(Vec<T>, usize);

impl<T> AsRef<[T]> for FromVecInner<T> {
    fn as_ref(&self) -> &[T] {
        self.0.as_ref()
    }
}

impl<T> Wrapper<Vec<T>> for FromVecInner<T> {
    fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<T: Clone> Producer for FromVecInner<T> {
    /// The type of the items to be produced.
    type Item = T;
    /// The final value emitted once the end of the slice has been reached.
    type Final = ();
    /// The producer can never error.
    type Error = !;

    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        if self.0.len() == self.1 {
            Ok(Either::Right(()))
        } else {
            let item = self.0[self.1].clone();
            self.1 += 1;

            Ok(Either::Left(item))
        }
    }
}

impl<T: Copy> BufferedProducer for FromVecInner<T> {
    fn slurp(&mut self) -> Result<(), Self::Error> {
        // There are no effects to perform so we simply return.
        Ok(())
    }
}

impl<T: Copy> BulkProducer for FromVecInner<T> {
    fn producer_slots(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
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

#[cfg(test)]
mod tests {
    use super::*;

    use core::mem::MaybeUninit;

    // Panic conditions:
    //
    // - `produce()` must not be called after final or error
    // - `slurp()` must not be called after final or error
    // - `producer_slots()` must not be called after final or error
    // - `did_produce()` must not be called after final or error
    // - `bulk_produce()` must not be called after final or error
    // - `did_produce(amount)` must not be called with `amount` greater that available slots

    // In each of the following tests, the final function call should panic.

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_produce_after_final() {
        let mut slice_producer = FromVec::new(b"ufo".to_vec());
        loop {
            // Call `produce()` until the final value is emitted.
            if let Ok(Either::Right(_)) = slice_producer.produce() {
                break;
            }
        }

        let _ = slice_producer.produce();
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_slurp_after_final() {
        let mut slice_producer = FromVec::new(b"ufo".to_vec());
        loop {
            if let Ok(Either::Right(_)) = slice_producer.produce() {
                break;
            }
        }

        let _ = slice_producer.slurp();
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_producer_slots_after_final() {
        let mut slice_producer = FromVec::new(b"ufo".to_vec());
        loop {
            if let Ok(Either::Right(_)) = slice_producer.produce() {
                break;
            }
        }

        let _ = slice_producer.producer_slots();
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_did_produce_after_final() {
        let mut slice_producer = FromVec::new(b"ufo".to_vec());
        loop {
            if let Ok(Either::Right(_)) = slice_producer.produce() {
                break;
            }
        }

        let _ = slice_producer.did_produce(3);
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_bulk_produce_after_final() {
        let mut slice_producer = FromVec::new(b"tofu".to_vec());
        loop {
            if let Ok(Either::Right(_)) = slice_producer.produce() {
                break;
            }
        }

        let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
        let _ = slice_producer.bulk_produce_maybeuninit(&mut buf);
    }

    #[test]
    #[should_panic(
        expected = "may not call `did_produce` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_produce_with_amount_greater_than_available_slots() {
        let mut slice_producer = FromVec::new(b"ufo".to_vec());

        let _ = slice_producer.did_produce(21);
    }
}
