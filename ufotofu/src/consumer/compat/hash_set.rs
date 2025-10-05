//! Consumer functionality for [`HashSet`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoConsumer`] impl for `HashSet<T>`,
//! - an [`IntoConsumer`] impl for `&mut HashSet<T>`.
//!
//! <br/>Counterpart: the [`producer::compat::btree_set`] module.

use core::fmt::Debug;

use core::hash::Hash;
use std::collections::HashSet;

use crate::prelude::*;

/// The consumer of the [`IntoConsumer`] impl of `HashSet<T>`; it inserts consumed items to the [`HashSet`].
///
/// Use the [`Into`] impl to recover the set when you are done consuming items.
///
/// ```
/// use ufotofu::prelude::*;
/// use std::collections::HashSet;
/// # pollster::block_on(async{
/// let mut c = HashSet::new().into_consumer();
///
/// c.consume(1).await?;
/// c.consume(4).await?;
/// c.consume(2).await?;
///
/// let mut set: HashSet<_> = c.into();
/// assert_eq!(set, vec![1, 2, 4].into_iter().collect::<HashSet<_>>());
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::hash_set::IntoProducer] type.
#[derive(Debug)]

pub struct IntoConsumer<T>(HashSet<T>);

impl<T> From<IntoConsumer<T>> for HashSet<T> {
    fn from(value: IntoConsumer<T>) -> Self {
        value.0
    }
}

impl<T: Hash + Eq> Consumer for IntoConsumer<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;

    /// Inserts the item into the set.
    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.0.insert(item);
        Ok(())
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!();
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T: Hash + Eq> crate::IntoConsumer for HashSet<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumer<T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumer(self)
    }
}

/// The consumer of the [`IntoConsumer`] impl of `&mut HashSet<T>`; it inserts consumed items to the [`HashSet`].
///
/// ```
/// use ufotofu::prelude::*;
/// use std::collections::HashSet;
/// # pollster::block_on(async{
/// let mut set = HashSet::new();
/// let mut c = (&mut set).into_consumer();
///
/// c.consume(1).await?;
/// c.consume(4).await?;
/// c.consume(2).await?;
///
/// assert_eq!(set, vec![1, 2, 4].into_iter().collect::<HashSet<_>>());
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::hash_set::IntoProducerRef] type.
#[derive(Debug)]

pub struct IntoConsumerMut<'a, T>(&'a mut HashSet<T>);

impl<'a, T: Hash + Eq> Consumer for IntoConsumerMut<'a, T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;

    /// Inserts the item into the set.
    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.0.insert(item);
        Ok(())
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!();
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a, T: Hash + Eq> crate::IntoConsumer for &'a mut HashSet<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumerMut<'a, T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumerMut(self)
    }
}
