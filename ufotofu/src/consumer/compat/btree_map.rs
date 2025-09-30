//! Consumer functionality for [`BTreeMap`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoConsumer`] impl for `BTreeMap<K, V>`,
//! - an [`IntoConsumer`] impl for `&mut BTreeMap<K, V>`.
//!
//! <br/>Counterpart: the [`producer::compat::btree_map`] module.

use core::fmt::Debug;

use std::collections::BTreeMap;

use crate::prelude::*;

/// The consumer of the [`IntoConsumer`] impl of `BTreeMap<K, V>`; it inserts key-value pairs to the [`BTreeMap`].
///
/// Use the [`Into`] impl to recover the map when you are done consuming items.
///
/// ```
/// use ufotofu::prelude::*;
/// use std::collections::BTreeMap;
/// # pollster::block_on(async{
/// let mut c = BTreeMap::new().into_consumer();
///
/// c.consume((1, 'a')).await?;
/// c.consume((4, 'q')).await?;
/// c.consume((2, 'b')).await?;
/// c.consume((4, 'd')).await?;
///
/// let mut map: BTreeMap<_, _> = c.into();
/// let collected: Vec<_> = map.into_iter().collect();
/// assert_eq!(collected, vec![(1, 'a'), (2, 'b'), (4, 'd')]);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::btree_map::IntoProducer] type.
#[derive(Debug)]

pub struct IntoConsumer<K, V>(BTreeMap<K, V>);

impl<K, V> From<IntoConsumer<K, V>> for BTreeMap<K, V> {
    fn from(value: IntoConsumer<K, V>) -> Self {
        value.0
    }
}

impl<K: Ord, V> Consumer for IntoConsumer<K, V> {
    type Item = (K, V);
    type Final = Infallible;
    type Error = Infallible;

    /// Inserts the key-value pair into the map.
    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.0.insert(item.0, item.1);
        Ok(())
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!();
    }
}

impl<K: Ord, V> crate::IntoConsumer for BTreeMap<K, V> {
    type Item = (K, V);
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumer<K, V>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumer(self)
    }
}

/// The consumer of the [`IntoConsumer`] impl of `&mut BTreeMap<K, V>`; it inserts consumed key-value pairs into the [`BTreeMap`].
///
/// ```
/// use ufotofu::prelude::*;
/// use std::collections::BTreeMap;
/// # pollster::block_on(async{
/// let mut map = BTreeMap::new();
/// let mut c = (&mut map).into_consumer();
///
/// c.consume((1, 'a')).await?;
/// c.consume((4, 'q')).await?;
/// c.consume((2, 'b')).await?;
/// c.consume((4, 'd')).await?;
///
/// let collected: Vec<_> = map.into_iter().collect();
/// assert_eq!(collected, vec![(1, 'a'), (2, 'b'), (4, 'd')]);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::btree_map::IntoProducerRef] type.
#[derive(Debug)]

pub struct IntoConsumerMut<'a, K, V>(&'a mut BTreeMap<K, V>);

impl<'a, K: Ord, V> Consumer for IntoConsumerMut<'a, K, V> {
    type Item = (K, V);
    type Final = Infallible;
    type Error = Infallible;

    /// Inserts the key-value pair into the map.
    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.0.insert(item.0, item.1);
        Ok(())
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!();
    }
}

impl<'a, K: Ord, V> crate::IntoConsumer for &'a mut BTreeMap<K, V> {
    type Item = (K, V);
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumerMut<'a, K, V>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumerMut(self)
    }
}
