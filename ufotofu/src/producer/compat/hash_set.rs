//! Producer functionality for [`HashSet`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoProducer`] impl for `HashSet<T>`, and
//! - an [`IntoProducer`] impl for `&HashSet<T>`.
//!
//! <br/>Counterpart: the [`consumer::compat::hash_set`] module.

use std::collections::HashSet;

use crate::{
    prelude::*,
    producer::compat::{iterator_to_producer, IteratorToProducer},
};

/// The producer of the [`IntoProducer`] impl of `HashSet<T>`.
///
/// ```
/// use std::collections::HashSet;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let set: HashSet<_> = vec![1, 2, 4].into_iter().collect();
/// let mut collected = HashSet::new();
/// let mut p = set.into_producer();
///
/// collected.insert(p.produce().await?.unwrap_left());
/// collected.insert(p.produce().await?.unwrap_left());
/// collected.insert(p.produce().await?.unwrap_left());
/// assert_eq!(p.produce().await?, Right(()));
/// assert_eq!(collected, vec![1, 2, 4].into_iter().collect::<HashSet<_>>());
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [`consumer::compat::hash_set::IntoConsumer`].
pub struct IntoProducer<T>(IteratorToProducer<<HashSet<T> as IntoIterator>::IntoIter>);

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

impl<T> crate::IntoProducer for HashSet<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducer<T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducer(iterator_to_producer(
            <HashSet<T> as IntoIterator>::into_iter(self),
        ))
    }
}

/// The producer of the [`IntoProducer`] impl of `&HashSet<T>`.
///
/// ```
/// use std::collections::HashSet;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let set: HashSet<_> = vec![1, 2, 4].into_iter().collect();
/// let mut collected = HashSet::new();
/// let mut p = (&set).into_producer();
///
/// collected.insert(p.produce().await?.unwrap_left());
/// collected.insert(p.produce().await?.unwrap_left());
/// collected.insert(p.produce().await?.unwrap_left());
/// assert_eq!(p.produce().await?, Right(()));
/// assert_eq!(collected, vec![&1, &2, &4].into_iter().collect::<HashSet<_>>());
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [`consumer::compat::hash_set::IntoConsumerMut`].
pub struct IntoProducerRef<'s, T>(IteratorToProducer<<&'s HashSet<T> as IntoIterator>::IntoIter>);

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

impl<'s, T> crate::IntoProducer for &'s HashSet<T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerRef<'s, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerRef(iterator_to_producer(self.iter()))
    }
}
