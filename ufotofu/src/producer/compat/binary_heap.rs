//! Producer functionality for [`BinaryHeap`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoProducer`] impl for `BinaryHeap<T>`, and
//! - an [`IntoProducer`] impl for `&BinaryHeap<T>`.
//!
//! <br/>Counterpart: the [`ufotofu::consumer::compat::vec`] module.

use std::collections::BinaryHeap;

use crate::{
    prelude::*,
    producer::compat::{iterator_to_producer, IteratorToProducer},
};

/// The producer of the [`IntoProducer`] impl of `BinaryHeap<T>`.
///
/// ```
/// use std::collections::BinaryHeap;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let heap: BinaryHeap<_> = vec![1, 2, 4].into_iter().collect();
/// let mut p = heap.into_producer();
///
/// assert_eq!(p.produce().await?, Left(4));
/// assert_eq!(p.produce().await?, Left(2));
/// assert_eq!(p.produce().await?, Left(1));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [TODO].
pub struct IntoProducerBinaryHeap<T>(IteratorToProducer<<BinaryHeap<T> as IntoIterator>::IntoIter>);

impl<T> Producer for IntoProducerBinaryHeap<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<T> crate::IntoProducer for BinaryHeap<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerBinaryHeap<T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerBinaryHeap(iterator_to_producer(
            <BinaryHeap<T> as IntoIterator>::into_iter(self),
        ))
    }
}

/// The producer of the [`IntoProducer`] impl of `&BinaryHeap<T>`.
///
/// ```
/// use std::collections::BinaryHeap;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let heap: BinaryHeap<_> = vec![1, 2, 4].into_iter().collect();
/// let mut p = (&heap).into_producer();
///
/// assert_eq!(p.produce().await?, Left(&4));
/// assert_eq!(p.produce().await?, Left(&2));
/// assert_eq!(p.produce().await?, Left(&1));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [TODO].
pub struct IntoProducerBinaryHeapRef<'s, T>(
    IteratorToProducer<<&'s BinaryHeap<T> as IntoIterator>::IntoIter>,
);

impl<'s, T> Producer for IntoProducerBinaryHeapRef<'s, T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<'s, T> crate::IntoProducer for &'s BinaryHeap<T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerBinaryHeapRef<'s, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerBinaryHeapRef(iterator_to_producer(self.into_iter()))
    }
}
