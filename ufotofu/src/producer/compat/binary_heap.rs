//! Producer functionality for [`BinaryHeap`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoProducer`] impl for `BinaryHeap<T>`, and
//! - an [`IntoProducer`] impl for `&BinaryHeap<T>`.
//!
//! <br/>Counterpart: the [`consumer::compat::binary_heap`] module.

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
/// <br/>Counterpart: [`consumer::compat::binary_heap::IntoConsumer`].
pub struct IntoProducer<T>(IteratorToProducer<<BinaryHeap<T> as IntoIterator>::IntoIter>);

impl<T> Producer for IntoProducer<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T> crate::IntoProducer for BinaryHeap<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducer<T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducer(iterator_to_producer(
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
/// <br/>Counterpart: [`consumer::compat::btree_set::IntoConsumerMut`].
pub struct IntoProducerRef<'s, T>(
    IteratorToProducer<<&'s BinaryHeap<T> as IntoIterator>::IntoIter>,
);

impl<'s, T> Producer for IntoProducerRef<'s, T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'s, T> crate::IntoProducer for &'s BinaryHeap<T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerRef<'s, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerRef(iterator_to_producer(self.iter()))
    }
}
