//! Producer functionality for [`Vec`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoProducer`] impl for `Vec<T>`,
//! - an [`IntoProducer`] impl for `&Vec<T>`, and
//! - an [`IntoProducer`] impl for `&mut Vec<T>`.
//!
//! <br/>Counterpart: the [`ufotofu::consumer::compat::vec`] module.

//////////////////////////////////////////////////////////////////////////////////////////////////////
// The consuming producer cannot be done properly on stable until Vec::into_raw_parts stabilises =( //
// Non-bulk boring implementation below for now.                                                    //
//////////////////////////////////////////////////////////////////////////////////////////////////////

use core::convert::Infallible;

use alloc::vec::Vec;

use crate::{
    prelude::*,
    producer::compat::{iterator_to_producer, IteratorToProducer},
};

/// The producer of the [`IntoProducer`] impl of [`Vec`].
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let v = vec![1, 2, 4];
/// let mut p = v.into_producer();
///
/// assert_eq!(p.produce().await?, Left(1));
/// assert_eq!(p.produce().await?, Left(2));
/// assert_eq!(p.produce().await?, Left(4));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [TODO].
pub struct IntoProducer<T>(IteratorToProducer<<Vec<T> as IntoIterator>::IntoIter>);

impl<T> Producer for IntoProducer<T> {
    type Item = T;

    type Final = ();

    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<T> crate::IntoProducer for Vec<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducer<T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducer(iterator_to_producer(self.into_iter()))
    }
}

/// The producer of the [`IntoProducer`] impl of `&Vec<T>`.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let vec_ref = &vec![1, 2, 4];
/// let mut p = vec_ref.into_producer();
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
pub struct IntoProducerRef<'s, T>(IteratorToProducer<<&'s Vec<T> as IntoIterator>::IntoIter>);

impl<'s, T> Producer for IntoProducerRef<'s, T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<'s, T> crate::IntoProducer for &'s Vec<T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerRef<'s, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerRef(iterator_to_producer(self.into_iter()))
    }
}

/// The producer of the [`IntoProducer`] impl of `&mut Vec<T>`.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut v = vec![1, 2, 4];
/// let mut p = (&mut v).into_producer();
///
/// assert_eq!(p.produce().await?, Left(&mut 1));
/// let mut mutable_ref = p.produce().await?.unwrap_left();
/// *mutable_ref = 17;
/// assert_eq!(p.produce().await?, Left(&mut 4));
/// assert_eq!(p.produce().await?, Right(()));
/// assert_eq!(v, vec![1, 17, 4]);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [TODO].
pub struct IntoProducerMut<'s, T>(IteratorToProducer<<&'s mut Vec<T> as IntoIterator>::IntoIter>);

impl<'s, T> Producer for IntoProducerMut<'s, T> {
    type Item = &'s mut T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<'s, T> crate::IntoProducer for &'s mut Vec<T> {
    type Item = &'s mut T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerMut<'s, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerMut(iterator_to_producer(self.into_iter()))
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////////
// The below is how the consuming producer *should* be implemented, allowing for `IntoBulkProducer`. //
// Blocked on https://github.com/rust-lang/rust/issues/65816                                         //
///////////////////////////////////////////////////////////////////////////////////////////////////////

// // Modified from `std::vec::IntoIter`.

// use core::{
//     convert::Infallible,
//     fmt,
//     marker::PhantomData,
//     mem::ManuallyDrop,
//     ptr::{self, NonNull},
//     slice,
// };

// use crate::Producer;

// macro_rules! non_null {
//     (mut $place:expr, $t:ident) => {{
//         unsafe { &mut *((&raw mut $place) as *mut NonNull<$t>) }
//     }};
//     ($place:expr, $t:ident) => {{
//         {
//             unsafe { *((&raw const $place) as *const NonNull<$t>) }
//         }
//     }};
// }
// // #![allow(unused_unsafe)]

// /// A producer that moves out of a vector.
// ///
// /// This `struct` is created by the `into_producer` method on [`Vec`](super::Vec)
// /// (provided by the [`IntoProducer`] trait).
// ///
// /// # Example
// ///
// /// ```
// /// let v = vec![0, 1, 2];
// /// let p: ufotofu::IntoProducer<_> = v.into_producer();
// /// ```
// pub struct IntoProducer<T> {
//     pub(super) buf: NonNull<T>,
//     pub(super) phantom: PhantomData<T>,
//     pub(super) cap: usize,
//     pub(super) ptr: NonNull<T>,
//     /// If T is a ZST, this is actually ptr+len. This encoding is picked so that
//     /// ptr == end is a quick test for the Iterator being empty, that works
//     /// for both ZST and non-ZST.
//     /// For non-ZSTs the pointer is treated as `NonNull<T>`
//     pub(super) end: *const T,
// }

// impl<T> crate::IntoProducer for alloc::vec::Vec<T> {
//     type Item = T;
//     type Final = ();
//     type Error = Infallible;
//     type IntoProducer = IntoProducer<T>;

//     fn into_producer(self) -> Self::IntoProducer {
//         unsafe {
//             let (data_ptr, len, cap) = self.into_raw_parts();
//             let buf = data_ptr.non_null();
//             let begin = buf.as_ptr();
//             let end = if size_of::<T>() == 0 {
//                 begin.wrapping_byte_add(len)
//             } else {
//                 begin.add(len) as *const T
//             };
//             IntoProducer {
//                 buf,
//                 phantom: PhantomData,
//                 cap,
//                 ptr: buf,
//                 end,
//             }
//         }
//     }
// }

// impl<T: fmt::Debug> fmt::Debug for IntoProducer<T> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         f.debug_tuple("IntoProducer")
//             .field(&self.as_slice())
//             .finish()
//     }
// }

// impl<T> IntoProducer<T> {
//     /// Returns the remaining items of this producer as a slice.
//     pub fn as_slice(&self) -> &[T] {
//         unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len()) }
//     }

//     /// Returns the remaining items of this iterator as a mutable slice.
//     ///
//     /// # Examples
//     ///
//     /// ```
//     /// let vec = vec!['a', 'b', 'c'];
//     /// let mut into_iter = vec.into_iter();
//     /// assert_eq!(into_iter.as_slice(), &['a', 'b', 'c']);
//     /// into_iter.as_mut_slice()[2] = 'z';
//     /// assert_eq!(into_iter.next().unwrap(), 'a');
//     /// assert_eq!(into_iter.next().unwrap(), 'b');
//     /// assert_eq!(into_iter.next().unwrap(), 'z');
//     /// ```
//     pub fn as_mut_slice(&mut self) -> &mut [T] {
//         unsafe { &mut *self.as_raw_mut_slice() }
//     }

//     fn as_raw_mut_slice(&mut self) -> *mut [T] {
//         ptr::slice_from_raw_parts_mut(self.ptr.as_ptr(), self.len())
//     }

//     /// Forgets to Drop the remaining elements while still allowing the backing allocation to be freed.
//     pub(crate) fn forget_remaining_elements(&mut self) {
//         // For the ZST case, it is crucial that we mutate `end` here, not `ptr`.
//         // `ptr` must stay aligned, while `end` may be unaligned.
//         self.end = self.ptr.as_ptr();
//     }

//     #[inline]
//     fn len(&self) -> usize {
//         let exact = if size_of::<T>() == 0 {
//             self.end.addr().wrapping_sub(self.ptr.as_ptr().addr())
//         } else {
//             unsafe { non_null!(self.end, T).offset_from_unsigned(self.ptr) }
//         };
//         exact
//     }
// }

// impl<T> AsRef<[T]> for IntoProducer<T> {
//     fn as_ref(&self) -> &[T] {
//         self.as_slice()
//     }
// }

// impl<T> Producer for IntoProducer<T> {
//     type Item = T;

//     #[inline]
//     fn next(&mut self) -> Option<T> {
//         let ptr = if size_of::<T>() == 0 {
//             if self.ptr.as_ptr() == self.end as *mut T {
//                 return None;
//             }
//             // `ptr` has to stay where it is to remain aligned, so we reduce the length by 1 by
//             // reducing the `end`.
//             self.end = self.end.wrapping_byte_sub(1);
//             self.ptr
//         } else {
//             if self.ptr == non_null!(self.end, T) {
//                 return None;
//             }
//             let old = self.ptr;
//             self.ptr = unsafe { old.add(1) };
//             old
//         };
//         Some(unsafe { ptr.read() })
//     }
// }

// impl<T: Clone + Clone> Clone for IntoProducer<T> {
//     fn clone(&self) -> Self {
//         self.as_slice().to_vec().into_producer()
//     }
// }

// impl<T> Drop for IntoProducer<T> {
//     fn drop(&mut self) {
//         struct DropGuard<'a, T>(&'a mut IntoProducer<T>);

//         impl<T> Drop for DropGuard<'_, T> {
//             fn drop(&mut self) {
//                 unsafe {
//                     // RawVec handles deallocation
//                     let _ = RawVec::from_nonnull_in(self.0.buf, self.0.cap);
//                 }
//             }
//         }

//         let guard = DropGuard(self);
//         // destroy the remaining elements
//         unsafe {
//             ptr::drop_in_place(guard.0.as_raw_mut_slice());
//         }
//         // now `guard` will be dropped and do the rest
//     }
// }
