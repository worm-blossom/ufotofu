//! Producer functionality for [`BTreeMap`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoProducer`] impl for `BTreeMap<K, V>`,
//! - an [`IntoProducer`] impl for `&BTreeMap<K, V>`, and
//! - an [`IntoProducer`] impl for `&mut BTreeMap<K, V>`.
//!
//! <br/>Counterpart: the [`ufotofu::consumer::compat::vec`] module.

use std::collections::BTreeMap;

use crate::{
    prelude::*,
    producer::compat::{iterator_to_producer, IteratorToProducer},
};

/// The producer of the [`IntoProducer`] impl of `BTreeMap<K, V>`.
///
/// ```
/// use std::collections::BTreeMap;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let map: BTreeMap<_, _> = vec![(1, 'a'), (2, 'b'), (4, 'd')].into_iter().collect();
/// let mut p = map.into_producer();
///
/// assert_eq!(p.produce().await?, Left((1, 'a')));
/// assert_eq!(p.produce().await?, Left((2, 'b')));
/// assert_eq!(p.produce().await?, Left((4, 'd')));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [TODO].
pub struct IntoProducerBTreeMap<K, V>(
    IteratorToProducer<<BTreeMap<K, V> as IntoIterator>::IntoIter>,
);

impl<K, V> Producer for IntoProducerBTreeMap<K, V> {
    type Item = (K, V);
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<K, V> crate::IntoProducer for BTreeMap<K, V> {
    type Item = (K, V);
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerBTreeMap<K, V>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerBTreeMap(iterator_to_producer(
            <BTreeMap<K, V> as IntoIterator>::into_iter(self),
        ))
    }
}

/// The producer of the [`IntoProducer`] impl of `&BTreeMap<K, V>`.
///
/// ```
/// use std::collections::BTreeMap;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let map: BTreeMap<_, _> = vec![(1, 'a'), (2, 'b'), (4, 'd')].into_iter().collect();
/// let mut p = (&map).into_producer();
///
/// assert_eq!(p.produce().await?, Left((&1, &'a')));
/// assert_eq!(p.produce().await?, Left((&2, &'b')));
/// assert_eq!(p.produce().await?, Left((&4, &'d')));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [TODO].
pub struct IntoProducerBTreeMapRef<'s, K, V>(
    IteratorToProducer<<&'s BTreeMap<K, V> as IntoIterator>::IntoIter>,
);

impl<'s, K, V> Producer for IntoProducerBTreeMapRef<'s, K, V> {
    type Item = (&'s K, &'s V);
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<'s, K, V> crate::IntoProducer for &'s BTreeMap<K, V> {
    type Item = (&'s K, &'s V);
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerBTreeMapRef<'s, K, V>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerBTreeMapRef(iterator_to_producer(self.into_iter()))
    }
}

/// The producer of the [`IntoProducer`] impl of `&mut BTreeMap<K, V>`.
///
/// ```
/// use std::collections::BTreeMap;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut map: BTreeMap<_, _> = vec![(1, 'a'), (2, 'b'), (4, 'd')].into_iter().collect();
/// let mut p = (&mut map).into_producer();
///
/// assert_eq!(p.produce().await?, Left((&1, &mut 'a')));
/// let (k, mut v) = p.produce().await?.unwrap_left();
/// *v = 'q';
/// assert_eq!(p.produce().await?, Left((&4, &mut 'd')));
/// assert_eq!(p.produce().await?, Right(()));
/// assert_eq!(map, vec![(1, 'a'), (2, 'q'), (4, 'd')].into_iter().collect::<BTreeMap<_, _>>());
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [TODO].
pub struct IntoProducerBTreeMapMut<'s, K, V>(
    IteratorToProducer<<&'s mut BTreeMap<K, V> as IntoIterator>::IntoIter>,
);

impl<'s, K, V> Producer for IntoProducerBTreeMapMut<'s, K, V> {
    type Item = (&'s K, &'s mut V);
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<'s, K, V> crate::IntoProducer for &'s mut BTreeMap<K, V> {
    type Item = (&'s K, &'s mut V);
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerBTreeMapMut<'s, K, V>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerBTreeMapMut(iterator_to_producer(self.into_iter()))
    }
}
