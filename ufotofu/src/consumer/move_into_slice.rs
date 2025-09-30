use core::cmp::min;
use core::convert::AsRef;
use core::fmt::Debug;

use crate::prelude::*;

/// A (bulk) consumer that fills (i.e., overwrites) a slice with consumed data.
///
/// See [`move_into_slice`].
///
/// <br/>Counterpart: the [producer::CloneFromSlice] type.
#[derive(Debug)]

pub struct MoveIntoSlice<'a, T>(&'a mut [T], usize);

/// Creates a (bulk) consumer that sequentially overwrites the data in the given slice.
///
/// ```
/// # use ufotofu::prelude::*;
/// use consumer::move_into_slice;
/// # pollster::block_on(async {
///
/// let mut buf = [0, 0, 0, 0];
/// let mut into_slice = move_into_slice(&mut buf[1..]);
///
/// into_slice.consume(1).await?;
/// into_slice.consume(2).await?;
/// into_slice.consume(4).await?;
/// assert_eq!(into_slice.consume(8).await, Err(()));
///
/// assert_eq!(buf, [0, 1, 2, 4]);
/// # Result::<(), ()>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::clone_from_slice] function.
pub fn move_into_slice<'a, T>(slice: &'a mut [T]) -> MoveIntoSlice<'a, T> {
    MoveIntoSlice(slice, 0)
}

impl<'a, T> MoveIntoSlice<'a, T> {
    /// Returns the offset into the slice at which the next item will be written.
    ///
    /// ```
    /// # use ufotofu::prelude::*;
    /// use consumer::move_into_slice;
    /// # pollster::block_on(async {
    ///
    /// let mut buf = [0, 0, 0, 0];
    /// let mut into_slice = move_into_slice(&mut buf[1..]);
    ///
    /// assert_eq!(into_slice.offset(), 0);
    /// into_slice.consume(1).await?;
    /// assert_eq!(into_slice.offset(), 1);
    /// into_slice.consume(2).await?;
    /// assert_eq!(into_slice.offset(), 2);
    /// into_slice.consume(4).await?;
    /// assert_eq!(into_slice.offset(), 3);
    /// assert_eq!(into_slice.consume(8).await, Err(()));
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    pub fn offset(&self) -> usize {
        self.1
    }

    /// Returns the subslice of items that have been consumed so far.
    ///
    /// ```
    /// # use ufotofu::prelude::*;
    /// use consumer::move_into_slice;
    /// # pollster::block_on(async {
    ///
    /// let mut buf = [0, 0, 0, 0];
    /// let mut into_slice = move_into_slice(&mut buf[1..]);
    ///
    /// assert!(into_slice.consumed().is_empty());
    /// into_slice.consume(1).await?;
    /// assert_eq!(into_slice.consumed(), &[1][..]);
    /// into_slice.consume(2).await?;
    /// assert_eq!(into_slice.consumed(), &[1, 2][..]);
    /// into_slice.consume(4).await?;
    /// assert_eq!(into_slice.consumed(), &[1, 2, 4][..]);
    /// assert_eq!(into_slice.consume(8).await, Err(()));
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    pub fn consumed(&self) -> &[T] {
        &self.0[..self.offset()]
    }

    /// Returns the mutable subslice of items that have been consumed so far.
    ///
    /// ```
    /// # use ufotofu::prelude::*;
    /// use consumer::move_into_slice;
    /// # pollster::block_on(async {
    ///
    /// let mut buf = [0, 0, 0, 0];
    /// let mut into_slice = move_into_slice(&mut buf[1..]);
    ///
    /// assert!(into_slice.consumed_mut().is_empty());
    /// into_slice.consume(1).await?;
    /// assert_eq!(into_slice.consumed_mut(), &mut [1][..]);
    /// into_slice.consume(2).await?;
    /// assert_eq!(into_slice.consumed_mut(), &mut [1, 2][..]);
    /// into_slice.consume(4).await?;
    /// assert_eq!(into_slice.consumed_mut(), &mut [1, 2, 4][..]);
    /// assert_eq!(into_slice.consume(8).await, Err(()));
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    pub fn consumed_mut(&mut self) -> &mut [T] {
        let offset = self.offset();
        &mut self.0[..offset]
    }

    /// Returns the subslice of items that have not been overwritten yet.
    ///
    /// ```
    /// # use ufotofu::prelude::*;
    /// use consumer::move_into_slice;
    /// # pollster::block_on(async {
    ///
    /// let mut buf = [0, 0, 0, 0];
    /// let mut into_slice = move_into_slice(&mut buf[1..]);
    ///
    /// assert_eq!(into_slice.remaining(), &[0, 0, 0][..]);
    /// into_slice.consume(1).await?;
    /// assert_eq!(into_slice.remaining(), &[0, 0][..]);
    /// into_slice.consume(2).await?;
    /// assert_eq!(into_slice.remaining(), &[0][..]);
    /// into_slice.consume(4).await?;
    /// assert!(into_slice.remaining().is_empty());
    /// assert_eq!(into_slice.consume(8).await, Err(()));
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    pub fn remaining(&self) -> &[T] {
        &self.0[self.offset()..]
    }

    /// Returns the mutable subslice of items that have not been overwritten yet.
    ///
    /// ```
    /// # use ufotofu::prelude::*;
    /// use consumer::move_into_slice;
    /// # pollster::block_on(async {
    ///
    /// let mut buf = [0, 0, 0, 0];
    /// let mut into_slice = move_into_slice(&mut buf[1..]);
    ///
    /// assert_eq!(into_slice.remaining_mut(), &mut [0, 0, 0][..]);
    /// into_slice.consume(1).await?;
    /// assert_eq!(into_slice.remaining_mut(), &mut [0, 0][..]);
    /// into_slice.consume(2).await?;
    /// assert_eq!(into_slice.remaining_mut(), &mut [0][..]);
    /// into_slice.consume(4).await?;
    /// assert!(into_slice.remaining_mut().is_empty());
    /// assert_eq!(into_slice.consume(8).await, Err(()));
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    pub fn remaining_mut(&mut self) -> &mut [T] {
        let offset = self.offset();
        &mut self.0[offset..]
    }

    /// Consumes `self` and returns the original reference to the slice.
    ///
    /// ```
    /// # use ufotofu::prelude::*;
    /// use consumer::move_into_slice;
    /// # pollster::block_on(async {
    ///
    /// let mut buf = [0, 0, 0, 0];
    /// let mut into_slice = move_into_slice(&mut buf[1..]);
    ///
    /// into_slice.consume(1).await?;
    /// into_slice.consume(2).await?;
    /// assert_eq!(into_slice.into_inner(), &mut [1, 2, 0][..]);
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    pub fn into_inner(self) -> &'a mut [T] {
        self.0
    }
}

impl<T> AsRef<[T]> for MoveIntoSlice<'_, T> {
    fn as_ref(&self) -> &[T] {
        self.0
    }
}

impl<T> AsMut<[T]> for MoveIntoSlice<'_, T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.0
    }
}

impl<T> Consumer for MoveIntoSlice<'_, T> {
    type Item = T;
    type Final = ();
    type Error = ();

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        if self.0.len() == self.1 {
            Err(())
        } else {
            self.0[self.1] = item;
            self.1 += 1;
            Ok(())
        }
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T: Clone> BulkConsumer for MoveIntoSlice<'_, T> {
    async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
        debug_assert_ne!(
            buf.len(),
            0,
            "Must not call bulk_consume with an empty buffer."
        );

        let amount = min(buf.len(), self.0.len() - self.1);

        if amount == 0 {
            Err(())
        } else {
            self.0[self.1..self.1 + amount].clone_from_slice(&buf[..amount]);
            self.1 += amount;
            Ok(amount)
        }
    }
}
