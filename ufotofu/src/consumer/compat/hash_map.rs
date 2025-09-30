//! Consumer functionality for [`HashMap`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoConsumer`] impl for `HashMap<K, V>`,
//! - an [`IntoConsumer`] impl for `&mut HashMap<K, V>`.
//!
//! <br/>Counterpart: the [`producer::compat::btree_map`] module.

use core::fmt::Debug;
use core::hash::Hash;

use std::collections::HashMap;

use crate::prelude::*;

/// The consumer of the [`IntoConsumer`] impl of `HashMap<K, V>`; it inserts key-value pairs to the [`HashMap`].
///
/// Use the [`Into`] impl to recover the map when you are done consuming items.
///
/// ```
/// use ufotofu::prelude::*;
/// use std::collections::HashMap;
/// # pollster::block_on(async{
/// let mut c = HashMap::new().into_consumer();
///
/// c.consume((1, 'a')).await?;
/// c.consume((4, 'q')).await?;
/// c.consume((2, 'b')).await?;
/// c.consume((4, 'd')).await?;
///
/// let mut map: HashMap<_, _> = c.into();
/// assert_eq!(map, vec![(1, 'a'), (2, 'b'), (4, 'd')].into_iter().collect::<HashMap<_, _>>());
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::btree_map::IntoProducer] type.
#[derive(Debug)]

pub struct IntoConsumer<K, V>(HashMap<K, V>);

impl<K, V> From<IntoConsumer<K, V>> for HashMap<K, V> {
    fn from(value: IntoConsumer<K, V>) -> Self {
        value.0
    }
}

impl<K: Hash + Eq, V> Consumer for IntoConsumer<K, V> {
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

impl<K: Hash + Eq, V> crate::IntoConsumer for HashMap<K, V> {
    type Item = (K, V);
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumer<K, V>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumer(self)
    }
}

/// The consumer of the [`IntoConsumer`] impl of `&mut HashMap<K, V>`; it inserts consumed key-value pairs into the [`HashMap`].
///
/// ```
/// use ufotofu::prelude::*;
/// use std::collections::HashMap;
/// # pollster::block_on(async{
/// let mut map = HashMap::new();
/// let mut c = (&mut map).into_consumer();
///
/// c.consume((1, 'a')).await?;
/// c.consume((4, 'q')).await?;
/// c.consume((2, 'b')).await?;
/// c.consume((4, 'd')).await?;
///
/// assert_eq!(map, vec![(1, 'a'), (2, 'b'), (4, 'd')].into_iter().collect::<HashMap<_, _>>());
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::btree_map::IntoProducerMut] type.
#[derive(Debug)]

pub struct IntoConsumerMut<'a, K, V>(&'a mut HashMap<K, V>);

impl<'a, K: Hash + Eq, V> Consumer for IntoConsumerMut<'a, K, V> {
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

impl<'a, K: Hash + Eq, V> crate::IntoConsumer for &'a mut HashMap<K, V> {
    type Item = (K, V);
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumerMut<'a, K, V>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumerMut(self)
    }
}
