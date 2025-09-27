//! Producer functionality for [`HashSet`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoProducer`] impl for `HashSet<T>`, and
//! - an [`IntoProducer`] impl for `&HashSet<T>`.
//!
//! <br/>Counterpart: the [`ufotofu::consumer::compat::vec`] module.

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
/// <br/>Counterpart: [TODO].
pub struct IntoProducerHashSet<T>(IteratorToProducer<<HashSet<T> as IntoIterator>::IntoIter>);

impl<T> Producer for IntoProducerHashSet<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<T> crate::IntoProducer for HashSet<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerHashSet<T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerHashSet(iterator_to_producer(
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
/// <br/>Counterpart: [TODO].
pub struct IntoProducerHashSetRef<'s, T>(
    IteratorToProducer<<&'s HashSet<T> as IntoIterator>::IntoIter>,
);

impl<'s, T> Producer for IntoProducerHashSetRef<'s, T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<'s, T> crate::IntoProducer for &'s HashSet<T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerHashSetRef<'s, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerHashSetRef(iterator_to_producer(self.into_iter()))
    }
}
