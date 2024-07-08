use core::convert::AsRef;

use either::Either;
use wrapper::Wrapper;

use crate::sync::producer::Invariant;
use crate::sync::{BufferedProducer, BulkProducer, Producer};

/// Produces data from a slice.
pub struct SliceProducer_<'a, T>(Invariant<SliceProducer<'a, T>>);

impl<'a, T: core::fmt::Debug> core::fmt::Debug for SliceProducer_<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a, T> SliceProducer_<'a, T> {
    /// Create a producer which produces the data in the given slice.
    pub fn new(slice: &'a [T]) -> SliceProducer_<'a, T> {
        // Wrap the inner slice producer in the invariant type.
        let invariant = Invariant::new(SliceProducer(slice, 0));

        SliceProducer_(invariant)
    }

    /// Return the offset into the slice at which the next item will be produced.
    pub fn get_offset(&self) -> usize {
        (self.0).as_ref().1
    }

    /// Return the subslice of items that have been produced so far.
    pub fn get_produced_so_far(&self) -> &[T] {
        &(self.0).as_ref().0[..self.get_offset()]
    }

    /// Return the subslice of items that have not been produced yet.
    pub fn get_not_yet_produced(&self) -> &[T] {
        &(self.0).as_ref().0[self.get_offset()..]
    }
}

impl<'a, T> AsRef<[T]> for SliceProducer_<'a, T> {
    fn as_ref(&self) -> &[T] {
        let inner = self.0.as_ref();
        inner.as_ref()
    }
}

impl<'a, T> Wrapper<&'a [T]> for SliceProducer_<'a, T> {
    fn into_inner(self) -> &'a [T] {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<'a, T: Clone> Producer for SliceProducer_<'a, T> {
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

impl<'a, T: Copy> BufferedProducer for SliceProducer_<'a, T> {
    fn slurp(&mut self) -> Result<(), Self::Error> {
        self.0.slurp()
    }
}

impl<'a, T: Copy> BulkProducer for SliceProducer_<'a, T> {
    fn expose_items(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        self.0.expose_items()
    }

    fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.consider_produced(amount)
    }
}

#[derive(Debug)]
struct SliceProducer<'a, T>(&'a [T], usize);

impl<'a, T> AsRef<[T]> for SliceProducer<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.0
    }
}

impl<'a, T> Wrapper<&'a [T]> for SliceProducer<'a, T> {
    fn into_inner(self) -> &'a [T] {
        self.0
    }
}

impl<'a, T: Clone> Producer for SliceProducer<'a, T> {
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
            // Clone the item from the slice at the given index.
            let item = self.0[self.1].clone();
            // Increment the item counter.
            self.1 += 1;

            Ok(Either::Left(item))
        }
    }
}

impl<'a, T: Copy> BufferedProducer for SliceProducer<'a, T> {
    fn slurp(&mut self) -> Result<(), Self::Error> {
        // There are no effects to perform so we simply return.
        Ok(())
    }
}

impl<'a, T: Copy> BulkProducer for SliceProducer<'a, T> {
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
        let mut slice_producer = SliceProducer::new(b"ufo");
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
        let mut slice_producer = SliceProducer::new(b"ufo");
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
        let mut slice_producer = SliceProducer::new(b"ufo");
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
        let mut slice_producer = SliceProducer::new(b"ufo");
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
        let mut slice_producer = SliceProducer::new(b"tofu");
        loop {
            if let Ok(Either::Right(_)) = slice_producer.produce() {
                break;
            }
        }

        let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
        let _ = slice_producer.bulk_produce_uninit(&mut buf);
    }

    #[test]
    #[should_panic(
        expected = "may not call `consider_produced` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_produce_with_amount_greater_than_available_slots() {
        let mut slice_producer = SliceProducer::new(b"ufo");

        let _ = slice_producer.consider_produced(21);
    }
}
