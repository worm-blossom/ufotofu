//! Producer functionality for [`HashMap`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoProducer`] impl for `HashMap<K, V>`,
//! - an [`IntoProducer`] impl for `&HashMap<K, V>`, and
//! - an [`IntoProducer`] impl for `&mut HashMap<K, V>`.
//!
//! <br/>Counterpart: the [`consumer::compat::hash_map`] module.

use std::collections::HashMap;

use crate::{
    prelude::*,
    producer::compat::{iterator_to_producer, IteratorToProducer},
};

/// The producer of the [`IntoProducer`] impl of `HashMap<K, V>`.
///
/// ```
/// use std::collections::HashMap;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let map: HashMap<_, _> = vec![(1, 'a'), (2, 'b'), (4, 'd')].into_iter().collect();
/// let mut collected = HashMap::new();
/// let mut p = map.into_producer();
///
/// let (k, v) = p.produce().await?.unwrap_left();
/// collected.insert(k, v);
/// let (k, v) = p.produce().await?.unwrap_left();
/// collected.insert(k, v);
/// let (k, v) = p.produce().await?.unwrap_left();
/// collected.insert(k, v);
///
/// assert_eq!(p.produce().await?, Right(()));
/// assert_eq!(
///     collected,
///     vec![(1, 'a'), (2, 'b'), (4, 'd')].into_iter().collect::<HashMap<_, _>>(),
/// );
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [`consumer::compat::hash_map::IntoConsumer`].
pub struct IntoProducer<K, V>(IteratorToProducer<<HashMap<K, V> as IntoIterator>::IntoIter>);

impl<K, V> Producer for IntoProducer<K, V> {
    type Item = (K, V);
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<K, V> crate::IntoProducer for HashMap<K, V> {
    type Item = (K, V);
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducer<K, V>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducer(iterator_to_producer(
            <HashMap<K, V> as IntoIterator>::into_iter(self),
        ))
    }
}

/// The producer of the [`IntoProducer`] impl of `&HashMap<K, V>`.
///
/// ```
/// use std::collections::HashMap;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let map: HashMap<_, _> = vec![(1, 'a'), (2, 'b'), (4, 'd')].into_iter().collect();
/// let mut collected = HashMap::new();
/// let mut p = (&map).into_producer();
///
/// let (k, v) = p.produce().await?.unwrap_left();
/// collected.insert(*k, v);
/// let (k, v) = p.produce().await?.unwrap_left();
/// collected.insert(*k, v);
/// let (k, v) = p.produce().await?.unwrap_left();
/// collected.insert(*k, v);
///
/// assert_eq!(p.produce().await?, Right(()));
/// assert_eq!(
///     collected,
///     vec![(1, &'a'), (2, &'b'), (4, &'d')].into_iter().collect::<HashMap<_, _>>(),
/// );
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [`consumer::compat::hash_map::IntoConsumerMut`].
pub struct IntoProducerRef<'s, K, V>(
    IteratorToProducer<<&'s HashMap<K, V> as IntoIterator>::IntoIter>,
);

impl<'s, K, V> Producer for IntoProducerRef<'s, K, V> {
    type Item = (&'s K, &'s V);
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'s, K, V> crate::IntoProducer for &'s HashMap<K, V> {
    type Item = (&'s K, &'s V);
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerRef<'s, K, V>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerRef(iterator_to_producer(self.iter()))
    }
}

/// The producer of the [`IntoProducer`] impl of `&mut HashMap<K, V>`.
///
/// ```
/// use std::collections::HashMap;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut map: HashMap<_, _> = vec![(1, 'a'), (2, 'b'), (4, 'd')].into_iter().collect();
/// let mut collected = HashMap::new();
/// let mut p = (&mut map).into_producer();
///
/// let (k, mut v) = p.produce().await?.unwrap_left();
/// *v = 'q';
/// collected.insert(*k, v);
/// let (k, mut v) = p.produce().await?.unwrap_left();
/// *v = 'q';
/// collected.insert(*k, v);
/// let (k, mut v) = p.produce().await?.unwrap_left();
/// *v = 'q';
/// collected.insert(*k, v);
///
/// assert_eq!(p.produce().await?, Right(()));
/// assert_eq!(
///     collected,
///     vec![(1, &mut 'q'), (2, &mut 'q'), (4, &mut 'q')].into_iter().collect::<HashMap<_, _>>(),
/// );
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: none, because you cannot consume into a collection behind an immutable reference.
pub struct IntoProducerMut<'s, K, V>(
    IteratorToProducer<<&'s mut HashMap<K, V> as IntoIterator>::IntoIter>,
);

impl<'s, K, V> Producer for IntoProducerMut<'s, K, V> {
    type Item = (&'s K, &'s mut V);
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'s, K, V> crate::IntoProducer for &'s mut HashMap<K, V> {
    type Item = (&'s K, &'s mut V);
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerMut<'s, K, V>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerMut(iterator_to_producer(self.iter_mut()))
    }
}
