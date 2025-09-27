use core::cmp::min;
use core::convert::{AsRef, Infallible};
use core::fmt::Debug;

use either::Either;

use crate::prelude::*;

/// A (bulk) producer that sequentially clones and produces the data in the given slice.
///
/// See [`clone_from_slice`].
/// ```
#[derive(Debug)]

pub struct CloneFromSlice<'a, T>(&'a [T], usize);

/// Creates a (bulk) producer that sequentially clones and produces the data in the given slice.
///
/// ```
/// # use ufotofu::prelude::*;
/// use ufotofu::producer::clone_from_slice;
/// # pollster::block_on(async {
///
/// let mut from_slice = clone_from_slice(&[1, 2, 3]);
///
/// assert_eq!(Left(1), from_slice.produce().await?);
/// assert_eq!(Left(2), from_slice.produce().await?);
/// assert_eq!(Left(3), from_slice.produce().await?);
/// assert_eq!(Right(()), from_slice.produce().await?);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
pub fn clone_from_slice<'a, T>(slice: &'a [T]) -> CloneFromSlice<'a, T> {
    CloneFromSlice(slice, 0)
}

impl<'a, T> CloneFromSlice<'a, T> {
    /// Returns the offset into the slice at which the next item will be produced.
    ///
    /// ```
    /// # use ufotofu::prelude::*;
    /// use ufotofu::producer::clone_from_slice;
    ///
    /// let mut from_slice = clone_from_slice(&[1, 2, 3]);
    ///
    /// # pollster::block_on(async {
    /// assert_eq!(0, from_slice.offset());
    /// assert_eq!(Left(1), from_slice.produce().await?);
    /// assert_eq!(1, from_slice.offset());
    /// assert_eq!(Left(2), from_slice.produce().await?);
    /// assert_eq!(2, from_slice.offset());
    /// assert_eq!(Left(3), from_slice.produce().await?);
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
    /// use ufotofu::producer::clone_from_slice;
    ///
    /// let mut from_slice = clone_from_slice(&[1, 2, 3]);
    ///
    /// # pollster::block_on(async {
    /// assert!(from_slice.produced().is_empty());
    /// assert_eq!(Left(1), from_slice.produce().await?);
    /// assert_eq!(&[1], from_slice.produced());
    /// assert_eq!(Left(2), from_slice.produce().await?);
    /// assert_eq!(&[1, 2], from_slice.produced());
    /// assert_eq!(Left(3), from_slice.produce().await?);
    /// assert_eq!(&[1, 2, 3], from_slice.produced());
    /// assert_eq!(Right(()), from_slice.produce().await?);
    /// assert_eq!(&[1, 2, 3], from_slice.produced());
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
    /// use ufotofu::producer::clone_from_slice;
    ///
    /// let mut from_slice = clone_from_slice(&[1, 2, 3]);
    ///
    /// # pollster::block_on(async {
    /// assert_eq!(&[1, 2, 3], from_slice.remaining());
    /// assert_eq!(Left(1), from_slice.produce().await?);
    /// assert_eq!(&[2, 3], from_slice.remaining());
    /// assert_eq!(Left(2), from_slice.produce().await?);
    /// assert_eq!(&[3], from_slice.remaining());
    /// assert_eq!(Left(3), from_slice.produce().await?);
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
    /// use ufotofu::producer::clone_from_slice;
    ///
    /// let mut from_slice = clone_from_slice(&[1, 2, 3]);
    ///
    /// # pollster::block_on(async {
    /// assert_eq!(Left(1), from_slice.produce().await?);
    /// assert_eq!(Left(2), from_slice.produce().await?);
    /// assert_eq!(&[1, 2, 3], from_slice.into_inner());
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
}

impl<T: Clone> BulkProducer for CloneFromSlice<'_, T> {
    async fn bulk_produce(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<Either<usize, Self::Final>, Self::Error> {
        let amount = min(buf.len(), self.0.len() - self.1);

        if amount == 0 {
            return Ok(Right(()));
        } else {
            (&mut buf[..amount]).clone_from_slice(&self.0[self.1..self.1 + amount]);
            self.1 += amount;
            return Ok(Left(amount));
        }
    }
}
