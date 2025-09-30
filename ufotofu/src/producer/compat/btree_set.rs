//! Producer functionality for [`BTreeSet`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoProducer`] impl for `BTreeSet<T>`, and
//! - an [`IntoProducer`] impl for `&BTreeSet<T>`.
//!
//! <br/>Counterpart: the [`consumer::compat::btree_set`] module.

use std::collections::BTreeSet;

use crate::{
    prelude::*,
    producer::compat::{iterator_to_producer, IteratorToProducer},
};

/// The producer of the [`IntoProducer`] impl of `BTreeSet<T>`.
///
/// ```
/// use std::collections::BTreeSet;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let set: BTreeSet<_> = vec![1, 2, 4].into_iter().collect();
/// let mut p = set.into_producer();
///
/// assert_eq!(p.produce().await?, Left(1));
/// assert_eq!(p.produce().await?, Left(2));
/// assert_eq!(p.produce().await?, Left(4));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [`consumer::compat::btree_set::IntoConsumer`].
pub struct IntoProducer<T>(IteratorToProducer<<BTreeSet<T> as IntoIterator>::IntoIter>);

impl<T> Producer for IntoProducer<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<T> crate::IntoProducer for BTreeSet<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducer<T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducer(iterator_to_producer(
            <BTreeSet<T> as IntoIterator>::into_iter(self),
        ))
    }
}

/// The producer of the [`IntoProducer`] impl of `&BTreeSet<T>`.
///
/// ```
/// use std::collections::BTreeSet;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let set: BTreeSet<_> = vec![1, 2, 4].into_iter().collect();
/// let mut p = (&set).into_producer();
///
/// assert_eq!(p.produce().await?, Left(&1));
/// assert_eq!(p.produce().await?, Left(&2));
/// assert_eq!(p.produce().await?, Left(&4));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [`consumer::compat::hash_set::IntoConsumerMut`].
pub struct IntoProducerRef<'s, T>(IteratorToProducer<<&'s BTreeSet<T> as IntoIterator>::IntoIter>);

impl<'s, T> Producer for IntoProducerRef<'s, T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<'s, T> crate::IntoProducer for &'s BTreeSet<T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerRef<'s, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerRef(iterator_to_producer(self.into_iter()))
    }
}
