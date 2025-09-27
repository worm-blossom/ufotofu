// Modified from `core::array::IntoIter`.

use core::{cmp::min, convert::Infallible, fmt, mem::ManuallyDrop};

use crate::prelude::*;

/// A producer that moves items out of an array.
///
/// This `struct` is created by the `into_producer` method on [`[T; N]`](https://doc.rust-lang.org/std/primitive.array.html)
/// (provided by the [`IntoProducer`] trait).
///
/// # Example
///
/// ```
/// use ufotofu::prelude::*;
/// let arr = [0, 1, 2];
/// let p = arr.into_producer();
/// ```
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
