use core::convert::{AsRef, Infallible};
use core::fmt::Debug;

use either::Either;

use crate::producer::Invariant;
use crate::{BufferedProducer, BulkProducer, Producer};

invarianted_producer_outer_type!(
    /// Produces data from a slice.
    FromSlice_ FromSlice <'a, T>
);

invarianted_impl_debug!(FromSlice_<'a, T: Debug>);

impl<'a, T> FromSlice_<'a, T> {
    /// Creates a producer that produces the data in the given slice.
    ///
    /// ```
    /// use either::Either::*;
    /// use ufotofu::producer::*;
    /// use ufotofu::*;
    ///
    /// let mut from_slice = FromSlice::new(&[1, 2, 3]);
    ///
    /// pollster::block_on(async {
    ///     assert_eq!(Ok(Left(1)), from_slice.produce().await);
    ///     assert_eq!(Ok(Left(2)), from_slice.produce().await);
    ///     assert_eq!(Ok(Left(3)), from_slice.produce().await);
    ///     assert_eq!(Ok(Right(())), from_slice.produce().await);
    /// });
    /// ```
    pub fn new(slice: &'a [T]) -> FromSlice_<'a, T> {
        let invariant = Invariant::new(FromSlice(slice, 0));
        FromSlice_(invariant)
    }

    /// Returns the offset into the slice at which the next item will be produced.
    ///
    /// ```
    /// use either::Either::*;
    /// use ufotofu::producer::*;
    /// use ufotofu::*;
    ///
    /// let mut from_slice = FromSlice::new(&[1, 2, 3]);
    ///
    /// pollster::block_on(async {
    ///     assert_eq!(0, from_slice.offset());
    ///     assert_eq!(Ok(Left(1)), from_slice.produce().await);
    ///     assert_eq!(1, from_slice.offset());
    ///     assert_eq!(Ok(Left(2)), from_slice.produce().await);
    ///     assert_eq!(2, from_slice.offset());
    ///     assert_eq!(Ok(Left(3)), from_slice.produce().await);
    ///     assert_eq!(3, from_slice.offset());
    ///     assert_eq!(Ok(Right(())), from_slice.produce().await);
    ///     assert_eq!(3, from_slice.offset());
    /// });
    /// ```
    pub fn offset(&self) -> usize {
        (self.0).as_ref().1
    }

    /// Returns the subslice of items that have been produced so far.
    ///
    /// ```
    /// use either::Either::*;
    /// use ufotofu::producer::*;
    /// use ufotofu::*;
    ///
    /// let mut from_slice = FromSlice::new(&[1, 2, 3]);
    ///
    /// pollster::block_on(async {
    ///     assert!(from_slice.produced_so_far().is_empty());
    ///     assert_eq!(Ok(Left(1)), from_slice.produce().await);
    ///     assert_eq!(&[1], from_slice.produced_so_far());
    ///     assert_eq!(Ok(Left(2)), from_slice.produce().await);
    ///     assert_eq!(&[1, 2], from_slice.produced_so_far());
    ///     assert_eq!(Ok(Left(3)), from_slice.produce().await);
    ///     assert_eq!(&[1, 2, 3], from_slice.produced_so_far());
    ///     assert_eq!(Ok(Right(())), from_slice.produce().await);
    ///     assert_eq!(&[1, 2, 3], from_slice.produced_so_far());
    /// });
    /// ```
    pub fn produced_so_far(&self) -> &[T] {
        &(self.0).as_ref().0[..self.offset()]
    }

    /// Returns the subslice of items that have not been produced yet.
    ///
    /// ```
    /// use either::Either::*;
    /// use ufotofu::producer::*;
    /// use ufotofu::*;
    ///
    /// let mut from_slice = FromSlice::new(&[1, 2, 3]);
    ///
    /// pollster::block_on(async {
    ///     assert_eq!(&[1, 2, 3], from_slice.not_yet_produced());
    ///     assert_eq!(Ok(Left(1)), from_slice.produce().await);
    ///     assert_eq!(&[2, 3], from_slice.not_yet_produced());
    ///     assert_eq!(Ok(Left(2)), from_slice.produce().await);
    ///     assert_eq!(&[3], from_slice.not_yet_produced());
    ///     assert_eq!(Ok(Left(3)), from_slice.produce().await);
    ///     assert!(from_slice.not_yet_produced().is_empty());
    ///     assert_eq!(Ok(Right(())), from_slice.produce().await);
    ///     assert!(from_slice.not_yet_produced().is_empty());
    /// });
    /// ```
    pub fn not_yet_produced(&self) -> &[T] {
        &(self.0).as_ref().0[self.offset()..]
    }

    /// Consumes `self` and returns the original reference to the slice.
    ///
    /// ```
    /// use either::Either::*;
    /// use ufotofu::producer::*;
    /// use ufotofu::*;
    ///
    /// let mut from_slice = FromSlice::new(&[1, 2, 3]);
    ///
    /// pollster::block_on(async {
    ///     assert_eq!(Ok(Left(1)), from_slice.produce().await);
    ///     assert_eq!(Ok(Left(2)), from_slice.produce().await);
    ///     assert_eq!(&[1, 2, 3], from_slice.into_inner());
    /// });
    /// ```
    pub fn into_inner(self) -> &'a [T] {
        self.0.into_inner().0
    }
}

invarianted_impl_as_ref!(FromSlice_<'a, T>; [T]);

invarianted_impl_producer!(FromSlice_<'a, T: Clone> Item T;
    /// Emitted once the end of the slice has been reached.
    Final ();
    Error Infallible
);
invarianted_impl_buffered_producer!(FromSlice_<'a, T: Clone>);
invarianted_impl_bulk_producer!(FromSlice_<'a, T: Clone>);

#[derive(Debug)]
struct FromSlice<'a, T>(&'a [T], usize);

impl<T> AsRef<[T]> for FromSlice<'_, T> {
    fn as_ref(&self) -> &[T] {
        self.0
    }
}

impl<T: Clone> Producer for FromSlice<'_, T> {
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

impl<T: Clone> BufferedProducer for FromSlice<'_, T> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        // There are no effects to perform so we simply return.
        Ok(())
    }
}

impl<T: Clone> BulkProducer for FromSlice<'_, T> {
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
