use core::convert::AsRef;
use std::vec::Vec;

use either::Either;
use wrapper::Wrapper;

use crate::sync::producer::Invariant;
use crate::sync::{BufferedProducer, BulkProducer, Producer};

/// Produces data from a slice.
pub struct FromVec_<T>(Invariant<FromVec<T>>);

impl<T: core::fmt::Debug> core::fmt::Debug for FromVec_<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> FromVec_<T> {
    /// Create a producer which produces the data in the given slice.
    pub fn new(v: Vec<T>) -> FromVec_<T> {
        let invariant = Invariant::new(FromVec(v, 0));

        FromVec_(invariant)
    }
}

impl<T> AsRef<[T]> for FromVec_<T> {
    fn as_ref(&self) -> &[T] {
        let inner = self.0.as_ref();
        inner.as_ref()
    }
}

impl<T> Wrapper<Vec<T>> for FromVec_<T> {
    fn into_inner(self) -> Vec<T> {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<T: Clone> Producer for FromVec_<T> {
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

impl<T: Copy> BufferedProducer for FromVec_<T> {
    fn slurp(&mut self) -> Result<(), Self::Error> {
        self.0.slurp()
    }
}

impl<T: Copy> BulkProducer for FromVec_<T> {
    fn expose_items(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        self.0.expose_items()
    }

    fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.consider_produced(amount)
    }
}

#[derive(Debug)]
struct FromVec<T>(Vec<T>, usize);

impl<T> AsRef<[T]> for FromVec<T> {
    fn as_ref(&self) -> &[T] {
        self.0.as_ref()
    }
}

impl<T> Wrapper<Vec<T>> for FromVec<T> {
    fn into_inner(self) -> Vec<T> {
        self.0
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
        if self.0.len() == self.1 {
            Ok(Either::Right(()))
        } else {
            let item = self.0[self.1].clone();
            self.1 += 1;

            Ok(Either::Left(item))
        }
    }
}

impl<T: Copy> BufferedProducer for FromVec<T> {
    fn slurp(&mut self) -> Result<(), Self::Error> {
        // There are no effects to perform so we simply return.
        Ok(())
    }
}

impl<T: Copy> BulkProducer for FromVec<T> {
    fn expose_items(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        let slice = &self.0[self.1..];
        if slice.is_empty() {
            Ok(Either::Right(()))
        } else {
            Ok(Either::Left(slice))
        }
    }

    fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.1 += amount;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::sync::*;

    use core::mem::MaybeUninit;

    use either::Either;

    // The debug output hides the internals of using semantically transparent wrappers.
    #[test]
    fn debug_output_hides_transparent_wrappers() {
        let prod = FromVec::new(b"ufo".to_vec());
        assert_eq!(format!("{:?}", prod), "FromVec([117, 102, 111], 0)");
    }

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

        let _ = slice_producer.expose_items();
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

        let _ = slice_producer.consider_produced(3);
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
        expected = "may not call `consider_produced` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_produce_with_amount_greater_than_available_slots() {
        let mut slice_producer = FromVec::new(b"ufo".to_vec());

        let _ = slice_producer.consider_produced(21);
    }
}
