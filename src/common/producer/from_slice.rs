use core::convert::AsRef;

use either::Either;
use wrapper::Wrapper;

use crate::common::producer::Invariant;
use crate::local_nb::{
    BufferedProducer as BufferedProducerLocalNb, BulkProducer as BulkProducerLocalNb,
    Producer as ProducerLocalNb,
};
use crate::sync::{BufferedProducer, BulkProducer, Producer};

/// Produces data from a slice.
pub struct FromSlice_<'a, T>(Invariant<FromSlice<'a, T>>);

impl<'a, T: core::fmt::Debug> core::fmt::Debug for FromSlice_<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a, T> FromSlice_<'a, T> {
    /// Create a producer which produces the data in the given slice.
    pub fn new(slice: &'a [T]) -> FromSlice_<'a, T> {
        // Wrap the inner slice producer in the invariant type.
        let invariant = Invariant::new(FromSlice(slice, 0));

        FromSlice_(invariant)
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

impl<'a, T> AsRef<[T]> for FromSlice_<'a, T> {
    fn as_ref(&self) -> &[T] {
        let inner = self.0.as_ref();
        inner.as_ref()
    }
}

impl<'a, T> Wrapper<&'a [T]> for FromSlice_<'a, T> {
    fn into_inner(self) -> &'a [T] {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<'a, T: Clone> Producer for FromSlice_<'a, T> {
    type Item = T;
    /// Emitted once the end of the slice has been reached.
    type Final = ();
    type Error = !;

    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        Producer::produce(&mut self.0)
    }
}

impl<'a, T: Copy> BufferedProducer for FromSlice_<'a, T> {
    fn slurp(&mut self) -> Result<(), Self::Error> {
        BufferedProducer::slurp(&mut self.0)
    }
}

impl<'a, T: Copy> BulkProducer for FromSlice_<'a, T> {
    fn expose_items(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        BulkProducer::expose_items(&mut self.0)
    }

    fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        BulkProducer::consider_produced(&mut self.0, amount)
    }
}

impl<'a, T: Clone> ProducerLocalNb for FromSlice_<'a, T> {
    type Item = T;
    /// Emitted once the end of the slice has been reached.
    type Final = ();
    type Error = !;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        ProducerLocalNb::produce(&mut self.0).await
    }
}

impl<'a, T: Copy> BufferedProducerLocalNb for FromSlice_<'a, T> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        BufferedProducerLocalNb::slurp(&mut self.0).await
    }
}

impl<'a, T: Copy> BulkProducerLocalNb for FromSlice_<'a, T> {
    async fn expose_items<'b>(
        &'b mut self,
    ) -> Result<Either<&'b [Self::Item], Self::Final>, Self::Error>
    where
        Self::Item: 'b,
    {
        BulkProducerLocalNb::expose_items(&mut self.0).await
    }

    async fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        BulkProducerLocalNb::consider_produced(&mut self.0, amount).await
    }
}

#[derive(Debug)]
struct FromSlice<'a, T>(&'a [T], usize);

impl<'a, T> AsRef<[T]> for FromSlice<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.0
    }
}

impl<'a, T> Wrapper<&'a [T]> for FromSlice<'a, T> {
    fn into_inner(self) -> &'a [T] {
        self.0
    }
}

impl<'a, T: Clone> Producer for FromSlice<'a, T> {
    type Item = T;
    type Final = ();
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

impl<'a, T: Copy> BufferedProducer for FromSlice<'a, T> {
    fn slurp(&mut self) -> Result<(), Self::Error> {
        // There are no effects to perform so we simply return.
        Ok(())
    }
}

impl<'a, T: Copy> BulkProducer for FromSlice<'a, T> {
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

impl<'a, T: Clone> ProducerLocalNb for FromSlice<'a, T> {
    type Item = T;
    type Final = ();
    type Error = !;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        Producer::produce(self)
    }
}

impl<'a, T: Copy> BufferedProducerLocalNb for FromSlice<'a, T> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        BufferedProducer::slurp(self)
    }
}

impl<'a, T: Copy> BulkProducerLocalNb for FromSlice<'a, T> {
    async fn expose_items<'b>(
        &'b mut self,
    ) -> Result<Either<&'b [Self::Item], Self::Final>, Self::Error>
    where
        Self::Item: 'b,
    {
        BulkProducer::expose_items(self)
    }

    async fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        BulkProducer::consider_produced(self, amount)
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
        let mut slice_producer = FromSlice::new(b"ufo");
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
        let mut slice_producer = FromSlice::new(b"ufo");
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
        let mut slice_producer = FromSlice::new(b"ufo");
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
        let mut slice_producer = FromSlice::new(b"ufo");
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
        let mut slice_producer = FromSlice::new(b"tofu");
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
        let mut slice_producer = FromSlice::new(b"ufo");

        let _ = slice_producer.consider_produced(21);
    }
}
