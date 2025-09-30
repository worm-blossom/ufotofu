//! Producer functionality for [`[T; N]`](core::array).
//!
//! Specifically, the module provides
//!
//! - an [`IntoProducer`] impl for `[T; N]`,
//! - an [`IntoProducer`] impl for `&[T; N]`, and
//! - an [`IntoProducer`] impl for `&mut [T; N]`.
//!
//! <br/>Counterpart: the [`consumer::compat::array`] module.

use core::{cmp::min, convert::Infallible, fmt, mem::ManuallyDrop};

use crate::{
    prelude::*,
    producer::compat::{iterator_to_producer, IteratorToProducer},
};

/// The producer of the [`IntoProducer`] impl of `[T; N]`.
///
/// Implements [`BulkProducer`].
///
/// # Example
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let arr = [1, 2, 4];
/// let mut p = arr.into_producer();
///
/// assert_eq!(p.produce().await?, Left(1));
/// assert_eq!(p.produce().await?, Left(2));
/// assert_eq!(p.produce().await?, Left(4));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [TODO] type.
pub struct IntoProducer<const N: usize, T> {
    arr: ManuallyDrop<[T; N]>,
    /// Core invariant: everything in `&self.arr[self.offset..]` we still have ownership of,
    /// everything before `self.offset` has been moved out of self already.
    offset: usize,
}

impl<const N: usize, T> crate::IntoProducer for [T; N] {
    type Item = T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducer<N, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducer::new(self)
    }
}

impl<const N: usize, T: fmt::Debug> fmt::Debug for IntoProducer<N, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoProducer")
            .field(&self.as_slice())
            .finish()
    }
}

impl<const N: usize, T> IntoProducer<N, T> {
    /// Creates a new producer over the given `array`.
    pub fn new(array: [T; N]) -> Self {
        Self {
            arr: ManuallyDrop::new(array),
            offset: 0,
        }
    }

    /// Returns the remaining items of this producer as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let arr = ['a', 'b', 'c'];
    /// let mut p = arr.into_producer();
    ///
    /// assert_eq!(p.as_slice(), &['a', 'b', 'c']);
    /// assert_eq!(p.produce().await?, Left('a'));
    /// assert_eq!(p.as_slice(), &['b', 'c']);
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    pub fn as_slice(&self) -> &[T] {
        &self.arr[self.offset..]
    }

    // Actually not providing this, because then bulk production can safely assume non-overlapping buffers.
    //
    // /// Returns the remaining items of this iterator as a mutable slice.
    // ///
    // /// # Examples
    // ///
    // /// ```
    // /// use ufotofu::prelude::*;
    // /// # pollster::block_on(async{
    // /// let arr = ['a', 'b', 'c'];
    // /// let mut p = arr.into_producer();
    // ///
    // /// assert_eq!(p.as_slice(), &['a', 'b', 'c']);
    // /// assert_eq!(p.produce().await?, 'a');
    // /// p.as_mut_slice()[1] = 'z';
    // /// assert_eq!(p.produce.await?, 'b');
    // /// assert_eq!(p.produce.await?, 'z');
    // /// # Result::<(), Infallible>::Ok(())
    // /// # });
    // /// ```
    // pub fn as_mut_slice(&mut self) -> &mut [T] {
    //     &mut self.arr[self.offset..]
    // }

    /// Returns the number of remaining items.
    ///
    /// # Examples
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let arr = ['a', 'b', 'c'];
    /// let mut p = arr.into_producer();
    ///
    /// assert_eq!(p.len(), 3);
    /// assert_eq!(p.produce().await?, Left('a'));
    /// assert_eq!(p.len(), 2);
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }
}

impl<const N: usize, T> AsRef<[T]> for IntoProducer<N, T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

// Actually not providing this, because then bulk production can safely assume non-overlapping buffers.
// impl<const N: usize, T> AsMut<[T]> for IntoProducer<N, T> {
//     fn as_mut(&mut self) -> &mut [T] {
//         self.as_mut_slice()
//     }
// }

impl<const N: usize, T> Producer for IntoProducer<N, T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<either::Either<Self::Item, Self::Final>, Self::Error> {
        if self.len() == 0 {
            Ok(Right(()))
        } else {
            let item_ref = &self.arr[self.offset];
            self.offset += 1;

            // SAFETY: Casting a reference yields a valid pointer.
            Ok(Left(unsafe { (item_ref as *const T).read() }))
        }
    }
}

impl<const N: usize, T> BulkProducer for IntoProducer<N, T> {
    async fn bulk_produce(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<Either<usize, Self::Final>, Self::Error> {
        debug_assert_ne!(
            buf.len(),
            0,
            "Must not call bulk_produce with an empty buffer."
        );

        let amount = min(buf.len(), self.len());

        if amount == 0 {
            return Ok(Right(()));
        } else {
            let to_move = &self.as_slice()[..amount];

            // SAFETY: slice references yield valid pointers.
            // The memory is not overlapping, because we do not expose any
            // mutable references to the array after we took ownership of it.
            unsafe { core::ptr::copy_nonoverlapping(to_move.as_ptr(), buf.as_mut_ptr(), amount) };

            self.offset += amount;
            return Ok(Left(amount));
        }
    }
}

impl<const N: usize, T> Drop for IntoProducer<N, T> {
    fn drop(&mut self) {
        // Since we store the array in a `ManuallyDrop`, none of its items are dropped by default.
        // That is good, we only need to explicitly drop those items which have not been
        // produced yet, i.e., those starting at `self.offset`.
        for unproduced in &mut self.arr[self.offset..] {
            // Safety: we have not moved these items away, so nothing else might
            // Drop or use them again.
            unsafe {
                core::ptr::drop_in_place(unproduced);
            }
        }
    }
}

/// The producer of the [`IntoProducer`] impl of `&[T; N]`.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let arr_ref = &[1, 2, 4];
/// let mut p = arr_ref.into_producer();
///
/// assert_eq!(p.produce().await?, Left(&1));
/// assert_eq!(p.produce().await?, Left(&2));
/// assert_eq!(p.produce().await?, Left(&4));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [TODO].
pub struct IntoProducerRef<'s, T, const N: usize>(
    IteratorToProducer<<&'s [T; N] as IntoIterator>::IntoIter>,
);

impl<'s, T, const N: usize> Producer for IntoProducerRef<'s, T, N> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<'s, T, const N: usize> crate::IntoProducer for &'s [T; N] {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerRef<'s, T, N>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerRef(iterator_to_producer(self.into_iter()))
    }
}

/// The producer of the [`IntoProducer`] impl of `&mut [T; N]`.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut arr = [1, 2, 4];
/// let mut p = (&mut arr).into_producer();
///
/// assert_eq!(p.produce().await?, Left(&mut 1));
/// let mut mutable_ref = p.produce().await?.unwrap_left();
/// *mutable_ref = 17;
/// assert_eq!(p.produce().await?, Left(&mut 4));
/// assert_eq!(p.produce().await?, Right(()));
/// assert_eq!(arr, [1, 17, 4]);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [TODO].
pub struct IntoProducerMut<'s, T, const N: usize>(
    IteratorToProducer<<&'s mut [T; N] as IntoIterator>::IntoIter>,
);

impl<'s, T, const N: usize> Producer for IntoProducerMut<'s, T, N> {
    type Item = &'s mut T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<'s, T, const N: usize> crate::IntoProducer for &'s mut [T; N] {
    type Item = &'s mut T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerMut<'s, T, N>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerMut(iterator_to_producer(self.into_iter()))
    }
}
