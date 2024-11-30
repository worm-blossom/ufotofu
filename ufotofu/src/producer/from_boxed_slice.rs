use core::convert::{AsRef, Infallible};
use core::fmt::Debug;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{
    alloc::{Allocator, Global},
    boxed::Box,
    vec::Vec,
};
#[cfg(feature = "std")]
use std::{boxed::Box, vec::Vec};

use either::Either;

use crate::producer::Invariant;
use crate::{BufferedProducer, BulkProducer, Producer};

#[derive(Clone)]
/// Takes ownership of a boxed slice and produces its items.
pub struct FromBoxedSlice_<T>(Invariant<FromBoxedSlice<T>>);

invarianted_impl_debug!(FromBoxedSlice_<T: Debug>);

impl<T> FromBoxedSlice_<T> {
    /// Creates a producer which produces the data in the given boxed slice.
    ///
    /// ```
    /// use either::Either::*;
    /// use ufotofu::producer::*;
    /// use ufotofu::*;
    ///
    /// let mut from_boxed_slice = FromBoxedSlice::new(vec![1, 2, 3].into_boxed_slice());
    ///
    /// pollster::block_on(async {
    ///     assert_eq!(Ok(Left(1)), from_boxed_slice.produce().await);
    ///     assert_eq!(Ok(Left(2)), from_boxed_slice.produce().await);
    ///     assert_eq!(Ok(Left(3)), from_boxed_slice.produce().await);
    ///     assert_eq!(Ok(Right(())), from_boxed_slice.produce().await);
    /// });
    /// ```
    pub fn new(v: Box<[T]>) -> FromBoxedSlice_<T> {
        let invariant = Invariant::new(FromBoxedSlice(v, 0));

        FromBoxedSlice_(invariant)
    }

    /// Creates a producer which produces the data in the given vector.
    ///
    /// ```
    /// use either::Either::*;
    /// use ufotofu::producer::*;
    /// use ufotofu::*;
    ///
    /// let mut from_boxed_slice = FromBoxedSlice::from_vec(vec![1, 2, 3]);
    ///
    /// pollster::block_on(async {
    ///     assert_eq!(Ok(Left(1)), from_boxed_slice.produce().await);
    ///     assert_eq!(Ok(Left(2)), from_boxed_slice.produce().await);
    ///     assert_eq!(Ok(Left(3)), from_boxed_slice.produce().await);
    ///     assert_eq!(Ok(Right(())), from_boxed_slice.produce().await);
    /// });
    /// ```
    pub fn from_vec(v: Vec<T>) -> FromBoxedSlice_<T> {
        let invariant = Invariant::new(FromBoxedSlice(v.into_boxed_slice(), 0));

        FromBoxedSlice_(invariant)
    }

    /// Returns the offset into the slice at which the next item will be produced.
    ///
    /// ```
    /// use either::Either::*;
    /// use ufotofu::producer::*;
    /// use ufotofu::*;
    ///
    /// let mut from_boxed_slice = FromBoxedSlice::new(vec![1, 2, 3].into_boxed_slice());
    ///
    /// pollster::block_on(async {
    ///     assert_eq!(0, from_boxed_slice.offset());
    ///     assert_eq!(Ok(Left(1)), from_boxed_slice.produce().await);
    ///     assert_eq!(1, from_boxed_slice.offset());
    ///     assert_eq!(Ok(Left(2)), from_boxed_slice.produce().await);
    ///     assert_eq!(2, from_boxed_slice.offset());
    ///     assert_eq!(Ok(Left(3)), from_boxed_slice.produce().await);
    ///     assert_eq!(3, from_boxed_slice.offset());
    ///     assert_eq!(Ok(Right(())), from_boxed_slice.produce().await);
    ///     assert_eq!(3, from_boxed_slice.offset());
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
    /// let mut from_boxed_slice = FromBoxedSlice::new(vec![1, 2, 3].into_boxed_slice());
    ///
    /// pollster::block_on(async {
    ///     assert!(from_boxed_slice.produced_so_far().is_empty());
    ///     assert_eq!(Ok(Left(1)), from_boxed_slice.produce().await);
    ///     assert_eq!(&[1], from_boxed_slice.produced_so_far());
    ///     assert_eq!(Ok(Left(2)), from_boxed_slice.produce().await);
    ///     assert_eq!(&[1, 2], from_boxed_slice.produced_so_far());
    ///     assert_eq!(Ok(Left(3)), from_boxed_slice.produce().await);
    ///     assert_eq!(&[1, 2, 3], from_boxed_slice.produced_so_far());
    ///     assert_eq!(Ok(Right(())), from_boxed_slice.produce().await);
    ///     assert_eq!(&[1, 2, 3], from_boxed_slice.produced_so_far());
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
    /// let mut from_boxed_slice = FromBoxedSlice::new(vec![1, 2, 3].into_boxed_slice());
    ///
    /// pollster::block_on(async {
    ///     assert_eq!(&[1, 2, 3], from_boxed_slice.not_yet_produced());
    ///     assert_eq!(Ok(Left(1)), from_boxed_slice.produce().await);
    ///     assert_eq!(&[2, 3], from_boxed_slice.not_yet_produced());
    ///     assert_eq!(Ok(Left(2)), from_boxed_slice.produce().await);
    ///     assert_eq!(&[3], from_boxed_slice.not_yet_produced());
    ///     assert_eq!(Ok(Left(3)), from_boxed_slice.produce().await);
    ///     assert!(from_boxed_slice.not_yet_produced().is_empty());
    ///     assert_eq!(Ok(Right(())), from_boxed_slice.produce().await);
    ///     assert!(from_boxed_slice.not_yet_produced().is_empty());
    /// });
    /// ```
    pub fn not_yet_produced(&self) -> &[T] {
        &(self.0).as_ref().0[self.offset()..]
    }

    /// Returns the full slice from which this was constructed.
    ///
    /// ```
    /// use either::Either::*;
    /// use ufotofu::producer::*;
    /// use ufotofu::*;
    ///
    /// let mut from_boxed_slice = FromBoxedSlice::new(vec![1, 2, 3].into_boxed_slice());
    ///
    /// pollster::block_on(async {
    ///     assert_eq!(Ok(Left(1)), from_boxed_slice.produce().await);
    ///     assert_eq!(Ok(Left(2)), from_boxed_slice.produce().await);
    ///     assert_eq!(vec![1, 2, 3].into_boxed_slice(), from_boxed_slice.into_inner());
    /// });
    /// ```
    pub fn into_inner(self) -> Box<[T]> {
        self.0.into_inner().0
    }
}

/// Returns a reference to the full slice.
impl<T> AsRef<[T]> for FromBoxedSlice_<T> {
    fn as_ref(&self) -> &[T] {
        self.0.as_ref().as_ref()
    }
}

invarianted_impl_producer!(FromBoxedSlice_<T: Clone> Item T;
    /// Emitted once the end of the boxed slice has been reached.
    Final ();
    Error Infallible
);
invarianted_impl_buffered_producer!(FromBoxedSlice_<T: Clone>);
invarianted_impl_bulk_producer!(FromBoxedSlice_<T: Clone>);

#[derive(Debug, Clone)]
struct FromBoxedSlice<T>(Box<[T]>, usize);

impl<T> AsRef<[T]> for FromBoxedSlice<T> {
    fn as_ref(&self) -> &[T] {
        self.0.as_ref()
    }
}

impl<T: Clone> Producer for FromBoxedSlice<T> {
    /// The type of the items to be produced.
    type Item = T;
    /// The final value emitted once the end of the slice has been reached.
    type Final = ();
    /// The producer can never error.
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

impl<T: Clone> BufferedProducer for FromBoxedSlice<T> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        // There are no effects to perform so we simply return.
        Ok(())
    }
}

impl<T: Clone> BulkProducer for FromBoxedSlice<T> {
    async fn expose_items<'a>(
        &'a mut self,
    ) -> Result<Either<&'a [Self::Item], Self::Final>, Self::Error>
    where
        Self::Item: 'a,
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
