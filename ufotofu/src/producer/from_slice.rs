use core::convert::{AsRef, Infallible};
use core::fmt::Debug;

use either::Either;
use wrapper::Wrapper;

use crate::producer::Invariant;
use crate::{BufferedProducer, BulkProducer, Producer};

invarianted_producer_outer_type!(
    /// Produces data from a slice.
    FromSlice_ FromSlice <'a, T>
);

invarianted_impl_debug!(FromSlice_<'a, T: Debug>);

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

invarianted_impl_as_ref!(FromSlice_<'a, T>; [T]);
invarianted_impl_wrapper!(FromSlice_<'a, T>; &'a [T]);

invarianted_impl_producer!(FromSlice_<'a, T: Clone> Item T;
    /// Emitted once the end of the slice has been reached.
    Final ();
    Error Infallible
);
invarianted_impl_buffered_producer!(FromSlice_<'a, T: Clone>);
invarianted_impl_bulk_producer!(FromSlice_<'a, T: Clone>);

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
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        if self.0.len() == self.1 {
            Ok(Either::Right(()))
        } else {
            let item = self.0[self.1].clone();
            self.1 += 1;

            Ok(Either::Left(item))
        }
    }
}

impl<'a, T: Clone> BufferedProducer for FromSlice<'a, T> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        // There are no effects to perform so we simply return.
        Ok(())
    }
}

impl<'a, T: Clone> BulkProducer for FromSlice<'a, T> {
    async fn expose_items<'b>(
        &'b mut self,
    ) -> Result<Either<&'b [Self::Item], Self::Final>, Self::Error>
    where
        Self::Item: 'b,
    {
        let slice = &self.0[self.1..];
        if slice.is_empty() {
            Ok(Either::Right(()))
        } else {
            Ok(Either::Left(slice))
        }
    }

    async fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.1 += amount;

        Ok(())
    }
}
