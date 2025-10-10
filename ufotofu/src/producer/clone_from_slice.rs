use core::convert::{AsRef, Infallible};
use core::fmt::Debug;
use core::marker::PhantomData;

use either::Either;

use crate::prelude::*;

/// A (bulk) producer that sequentially clones and produces the data in a slice.
///
/// See [`clone_from_slice`].
///
/// <br/>Counterpart: the [`consumer::MoveIntoSlice`] type.
#[derive(Debug, Clone)]

pub struct CloneFromSlice<'a, T>(&'a [T], usize);

/// Creates a (bulk) producer that sequentially clones and produces the data in the given slice.
///
/// ```
/// # use ufotofu::prelude::*;
/// use producer::clone_from_slice;
/// # pollster::block_on(async {
///
/// let mut from_slice = clone_from_slice(&[1, 2, 4]);
///
/// assert_eq!(from_slice.produce().await?, Left(1));
/// assert_eq!(from_slice.produce().await?, Left(2));
/// assert_eq!(from_slice.produce().await?, Left(4));
/// assert_eq!(from_slice.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [`consumer::move_into_slice`] function.
pub fn clone_from_slice<'a, T>(slice: &'a [T]) -> CloneFromSlice<'a, T> {
    CloneFromSlice(slice, 0)
}

impl<'a, T> CloneFromSlice<'a, T> {
    /// Returns the offset into the slice at which the next item will be produced.
    ///
    /// ```
    /// # use ufotofu::prelude::*;
    /// use producer::clone_from_slice;
    ///
    /// let mut from_slice = clone_from_slice(&[1, 2, 4]);
    ///
    /// # pollster::block_on(async {
    /// assert_eq!(0, from_slice.offset());
    /// assert_eq!(Left(1), from_slice.produce().await?);
    /// assert_eq!(1, from_slice.offset());
    /// assert_eq!(Left(2), from_slice.produce().await?);
    /// assert_eq!(2, from_slice.offset());
    /// assert_eq!(Left(4), from_slice.produce().await?);
    /// assert_eq!(3, from_slice.offset());
    /// assert_eq!(Right(()), from_slice.produce().await?);
    /// assert_eq!(3, from_slice.offset());
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    pub fn offset(&self) -> usize {
        self.1
    }

    /// Returns the subslice of items that have been produced so far.
    ///
    /// ```
    /// # use ufotofu::prelude::*;
    /// use producer::clone_from_slice;
    ///
    /// let mut from_slice = clone_from_slice(&[1, 2, 4]);
    ///
    /// # pollster::block_on(async {
    /// assert!(from_slice.produced().is_empty());
    /// assert_eq!(Left(1), from_slice.produce().await?);
    /// assert_eq!(&[1], from_slice.produced());
    /// assert_eq!(Left(2), from_slice.produce().await?);
    /// assert_eq!(&[1, 2], from_slice.produced());
    /// assert_eq!(Left(4), from_slice.produce().await?);
    /// assert_eq!(&[1, 2, 4], from_slice.produced());
    /// assert_eq!(Right(()), from_slice.produce().await?);
    /// assert_eq!(&[1, 2, 4], from_slice.produced());
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    pub fn produced(&self) -> &[T] {
        &self.0[..self.offset()]
    }

    /// Returns the subslice of items that have not been produced yet.
    ///
    /// ```
    /// # use ufotofu::prelude::*;
    /// use producer::clone_from_slice;
    ///
    /// let mut from_slice = clone_from_slice(&[1, 2, 4]);
    ///
    /// # pollster::block_on(async {
    /// assert_eq!(&[1, 2, 4], from_slice.remaining());
    /// assert_eq!(Left(1), from_slice.produce().await?);
    /// assert_eq!(&[2, 4], from_slice.remaining());
    /// assert_eq!(Left(2), from_slice.produce().await?);
    /// assert_eq!(&[4], from_slice.remaining());
    /// assert_eq!(Left(4), from_slice.produce().await?);
    /// assert!(from_slice.remaining().is_empty());
    /// assert_eq!(Right(()), from_slice.produce().await?);
    /// assert!(from_slice.remaining().is_empty());
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    pub fn remaining(&self) -> &[T] {
        &self.0[self.offset()..]
    }

    /// Consumes `self` and returns the original reference to the slice.
    ///
    /// ```
    /// # use ufotofu::prelude::*;
    /// use producer::clone_from_slice;
    ///
    /// let mut from_slice = clone_from_slice(&[1, 2, 4]);
    ///
    /// # pollster::block_on(async {
    /// assert_eq!(Left(1), from_slice.produce().await?);
    /// assert_eq!(Left(2), from_slice.produce().await?);
    /// assert_eq!(&[1, 2, 4], from_slice.into_inner());
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    pub fn into_inner(self) -> &'a [T] {
        self.0
    }
}

impl<T> AsRef<[T]> for CloneFromSlice<'_, T> {
    fn as_ref(&self) -> &[T] {
        self.0
    }
}

impl<T: Clone> Producer for CloneFromSlice<'_, T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        if self.0.len() == self.1 {
            Ok(Right(()))
        } else {
            let item = self.0[self.1].clone();
            self.1 += 1;

            Ok(Left(item))
        }
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T: Clone> BulkProducer for CloneFromSlice<'_, T> {
    async fn expose_items<F, R>(&mut self, f: F) -> Result<Either<R, Self::Final>, Self::Error>
    where
        F: AsyncFnOnce(&[Self::Item]) -> (usize, R),
    {
        let len = self.0.len() - self.1;

        if len == 0 {
            return Ok(Right(()));
        } else {
            let (amount, ret) = f(&self.0[self.1..self.1 + len]).await;
            assert!(amount <= len);
            self.1 += amount;

            Ok(Left(ret))
        }
    }
}

/// A (bulk) producer that sequentially clones and produces the data in a value implementing `AsRef<[T]>`.
///
/// See [`clone_from_owned_slice`].
///
/// <br/>Counterpart: the [`consumer::MoveIntoOwnedSlice`] type.
#[derive(Debug, Clone)]

pub struct CloneFromOwnedSlice<S, T>(S, usize, PhantomData<T>);

/// Creates a (bulk) producer that sequentially clones and produces the data in the given value implementing `AsRef<[T]>`.
///
/// ```
/// # use ufotofu::prelude::*;
/// use producer::clone_from_owned_slice;
/// # pollster::block_on(async {
///
/// let mut from_slice = clone_from_owned_slice([1, 2, 4]);
///
/// assert_eq!(from_slice.produce().await?, Left(1));
/// assert_eq!(from_slice.produce().await?, Left(2));
/// assert_eq!(from_slice.produce().await?, Left(4));
/// assert_eq!(from_slice.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [`consumer::move_into_owned_slice`] function.
pub fn clone_from_owned_slice<S, T>(owned_slice: S) -> CloneFromOwnedSlice<S, T> {
    CloneFromOwnedSlice(owned_slice, 0, PhantomData)
}

impl<S, T> CloneFromOwnedSlice<S, T>
where
    S: AsRef<[T]>,
{
    /// Returns the offset into the slice at which the next item will be produced.
    ///
    /// ```
    /// # use ufotofu::prelude::*;
    /// use producer::clone_from_owned_slice;
    ///
    /// let mut from_slice = clone_from_owned_slice([1, 2, 4]);
    ///
    /// # pollster::block_on(async {
    /// assert_eq!(0, from_slice.offset());
    /// assert_eq!(Left(1), from_slice.produce().await?);
    /// assert_eq!(1, from_slice.offset());
    /// assert_eq!(Left(2), from_slice.produce().await?);
    /// assert_eq!(2, from_slice.offset());
    /// assert_eq!(Left(4), from_slice.produce().await?);
    /// assert_eq!(3, from_slice.offset());
    /// assert_eq!(Right(()), from_slice.produce().await?);
    /// assert_eq!(3, from_slice.offset());
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    pub fn offset(&self) -> usize {
        self.1
    }

    /// Returns the subslice of items that have been produced so far.
    ///
    /// ```
    /// # use ufotofu::prelude::*;
    /// use producer::clone_from_owned_slice;
    ///
    /// let mut from_slice = clone_from_owned_slice([1, 2, 4]);
    ///
    /// # pollster::block_on(async {
    /// assert!(from_slice.produced().is_empty());
    /// assert_eq!(Left(1), from_slice.produce().await?);
    /// assert_eq!(&[1], from_slice.produced());
    /// assert_eq!(Left(2), from_slice.produce().await?);
    /// assert_eq!(&[1, 2], from_slice.produced());
    /// assert_eq!(Left(4), from_slice.produce().await?);
    /// assert_eq!(&[1, 2, 4], from_slice.produced());
    /// assert_eq!(Right(()), from_slice.produce().await?);
    /// assert_eq!(&[1, 2, 4], from_slice.produced());
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    pub fn produced(&self) -> &[T] {
        &self.0.as_ref()[..self.offset()]
    }

    /// Returns the subslice of items that have not been produced yet.
    ///
    /// ```
    /// # use ufotofu::prelude::*;
    /// use producer::clone_from_owned_slice;
    ///
    /// let mut from_slice = clone_from_owned_slice([1, 2, 4]);
    ///
    /// # pollster::block_on(async {
    /// assert_eq!(&[1, 2, 4], from_slice.remaining());
    /// assert_eq!(Left(1), from_slice.produce().await?);
    /// assert_eq!(&[2, 4], from_slice.remaining());
    /// assert_eq!(Left(2), from_slice.produce().await?);
    /// assert_eq!(&[4], from_slice.remaining());
    /// assert_eq!(Left(4), from_slice.produce().await?);
    /// assert!(from_slice.remaining().is_empty());
    /// assert_eq!(Right(()), from_slice.produce().await?);
    /// assert!(from_slice.remaining().is_empty());
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    pub fn remaining(&self) -> &[T] {
        &self.0.as_ref()[self.offset()..]
    }

    /// Consumes `self` and returns the original owned slice.
    ///
    /// ```
    /// # use ufotofu::prelude::*;
    /// use producer::clone_from_owned_slice;
    ///
    /// let mut from_slice = clone_from_owned_slice([1, 2, 4]);
    ///
    /// # pollster::block_on(async {
    /// assert_eq!(Left(1), from_slice.produce().await?);
    /// assert_eq!(Left(2), from_slice.produce().await?);
    /// assert_eq!([1, 2, 4], from_slice.into_inner());
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    pub fn into_inner(self) -> S {
        self.0
    }
}

impl<S, T> AsRef<[T]> for CloneFromOwnedSlice<S, T>
where
    S: AsRef<[T]>,
{
    fn as_ref(&self) -> &[T] {
        self.0.as_ref()
    }
}

impl<S, T: Clone> Producer for CloneFromOwnedSlice<S, T>
where
    S: AsRef<[T]>,
{
    type Item = T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        if self.0.as_ref().len() == self.1 {
            Ok(Right(()))
        } else {
            let item = self.0.as_ref()[self.1].clone();
            self.1 += 1;

            Ok(Left(item))
        }
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<S, T: Clone> BulkProducer for CloneFromOwnedSlice<S, T>
where
    S: AsRef<[T]>,
{
    async fn expose_items<F, R>(&mut self, f: F) -> Result<Either<R, Self::Final>, Self::Error>
    where
        F: AsyncFnOnce(&[Self::Item]) -> (usize, R),
    {
        let len = self.0.as_ref().len() - self.1;

        if len == 0 {
            return Ok(Right(()));
        } else {
            let (amount, ret) = f(&self.0.as_ref()[self.1..self.1 + len]).await;
            assert!(amount <= len);
            self.1 += amount;

            Ok(Left(ret))
        }
    }
}
