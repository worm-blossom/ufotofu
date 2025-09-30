//! Producer functionality for [`[T]`](core::slice).
//!
//! Specifically, the module provides
//!
//! - an [`IntoProducer`] impl for `&[T]`,
//! - an [`IntoProducer`] impl for `&mut [T]`,
//! - an [`IntoProducer`] impl for `Box<[T]>`,
//! - an [`IntoProducer`] impl for `&Box<[T]>`, and
//! - an [`IntoProducer`] impl for `&mut Box<[T]>`.
//!
//! <br/>Counterpart: the [`consumer::compat::slice`] module.

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

use crate::{
    prelude::*,
    producer::compat::{iterator_to_producer, IteratorToProducer},
};

/// The producer of the [`IntoProducer`] impl of `&[T]`.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let slice = &[1, 2, 4][..];
/// let mut p = slice.into_producer();
///
/// assert_eq!(p.produce().await?, Left(&1));
/// assert_eq!(p.produce().await?, Left(&2));
/// assert_eq!(p.produce().await?, Left(&4));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [`consumer::compat::slice::IntoConsumerMut`].
pub struct IntoProducerRef<'s, T>(IteratorToProducer<<&'s [T] as IntoIterator>::IntoIter>);

impl<'s, T> Producer for IntoProducerRef<'s, T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<'s, T> crate::IntoProducer for &'s [T] {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerRef<'s, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerRef(iterator_to_producer(self.iter()))
    }
}

/// The producer of the [`IntoProducer`] impl of `&mut [T]`.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let slice = &mut [1, 2, 4][..];
/// let mut p = slice.into_producer();
///
/// assert_eq!(p.produce().await?, Left(&mut 1));
/// let mut mutable_ref = p.produce().await?.unwrap_left();
/// *mutable_ref = 17;
/// assert_eq!(p.produce().await?, Left(&mut 4));
/// assert_eq!(p.produce().await?, Right(()));
/// assert_eq!(slice, &[1, 17, 4]);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: none, because you cannot consume into a slice behind an immutable reference.
pub struct IntoProducerMut<'s, T>(IteratorToProducer<<&'s mut [T] as IntoIterator>::IntoIter>);

impl<'s, T> Producer for IntoProducerMut<'s, T> {
    type Item = &'s mut T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<'s, T> crate::IntoProducer for &'s mut [T] {
    type Item = &'s mut T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerMut<'s, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerMut(iterator_to_producer(self.iter_mut()))
    }
}

/// The producer of the [`IntoProducer`] impl of `Box<[T]>`.
///
/// When [`vec_into_raw_parts`](https://github.com/rust-lang/rust/issues/65816), this type will provide a [`BulkProducer`] implementation.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let boxed_slice = vec![1, 2, 4].into_boxed_slice();
/// let mut p = boxed_slice.into_producer();
///
/// assert_eq!(p.produce().await?, Left(1));
/// assert_eq!(p.produce().await?, Left(2));
/// assert_eq!(p.produce().await?, Left(4));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [`consumer::compat::slice::IntoConsumerBoxed`].
#[cfg(feature = "alloc")]
pub struct IntoProducerBoxed<T>(IteratorToProducer<<Box<[T]> as IntoIterator>::IntoIter>);

#[cfg(feature = "alloc")]
impl<T> Producer for IntoProducerBoxed<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

#[cfg(feature = "alloc")]
impl<T> crate::IntoProducer for Box<[T]> {
    type Item = T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerBoxed<T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerBoxed(iterator_to_producer(<Box<[T]> as IntoIterator>::into_iter(
            self,
        )))
    }
}

/// The producer of the [`IntoProducer`] impl of `&Box<[T]>`.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let box_ref = &vec![1, 2, 4].into_boxed_slice();
/// let mut p = box_ref.into_producer();
///
/// assert_eq!(p.produce().await?, Left(&1));
/// assert_eq!(p.produce().await?, Left(&2));
/// assert_eq!(p.produce().await?, Left(&4));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [`consumer::compat::slice::IntoConsumerBoxedMut`].
#[cfg(feature = "alloc")]
#[allow(clippy::borrowed_box)]
pub struct IntoProducerBoxedRef<'s, T>(
    IteratorToProducer<<&'s Box<[T]> as IntoIterator>::IntoIter>,
);

#[cfg(feature = "alloc")]
impl<'s, T> Producer for IntoProducerBoxedRef<'s, T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

#[cfg(feature = "alloc")]
impl<'s, T> crate::IntoProducer for &'s Box<[T]> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerBoxedRef<'s, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerBoxedRef(iterator_to_producer(self.into_iter()))
    }
}

/// The producer of the [`IntoProducer`] impl of `&mut Box<[T]>`.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut boxed = vec![1, 2, 4].into_boxed_slice();
/// let mut p = (&mut boxed).into_producer();
///
/// assert_eq!(p.produce().await?, Left(&mut 1));
/// let mut mutable_ref = p.produce().await?.unwrap_left();
/// *mutable_ref = 17;
/// assert_eq!(p.produce().await?, Left(&mut 4));
/// assert_eq!(p.produce().await?, Right(()));
/// assert_eq!(boxed, vec![1, 17, 4].into_boxed_slice());
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: none, because you cannot consume into a slice behind an immutable reference.
#[cfg(feature = "alloc")]
pub struct IntoProducerBoxedMut<'s, T>(
    IteratorToProducer<<&'s mut Box<[T]> as IntoIterator>::IntoIter>,
);

#[cfg(feature = "alloc")]
impl<'s, T> Producer for IntoProducerBoxedMut<'s, T> {
    type Item = &'s mut T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

#[cfg(feature = "alloc")]
impl<'s, T> crate::IntoProducer for &'s mut Box<[T]> {
    type Item = &'s mut T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerBoxedMut<'s, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerBoxedMut(iterator_to_producer(self.into_iter()))
    }
}
