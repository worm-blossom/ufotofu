//! Consumer functionality for [`BTreeSet`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoConsumer`] impl for `BTreeSet<T>`,
//! - an [`IntoConsumer`] impl for `&mut BTreeSet<T>`.
//!
//! <br/>Counterpart: the [`producer::compat::btree_set`] module.

use core::fmt::Debug;

use std::collections::BTreeSet;

use crate::prelude::*;

/// The consumer of the [`IntoConsumer`] impl of `BTreeSet<T>`; it inserts consumed items to the [`BTreeSet`].
///
/// Use the [`Into`] impl to recover the set when you are done consuming items.
///
/// ```
/// use ufotofu::prelude::*;
/// use std::collections::BTreeSet;
/// # pollster::block_on(async{
/// let mut c = BTreeSet::new().into_consumer();
///
/// c.consume(1).await?;
/// c.consume(4).await?;
/// c.consume(2).await?;
///
/// let mut set: BTreeSet<_> = c.into();
/// let collected: Vec<_> = set.into_iter().collect();
/// assert_eq!(collected, vec![1, 2, 4]);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::btree_set::IntoProducer] type.
#[derive(Debug)]

pub struct IntoConsumer<T>(BTreeSet<T>);

impl<T> From<IntoConsumer<T>> for BTreeSet<T> {
    fn from(value: IntoConsumer<T>) -> Self {
        value.0
    }
}

impl<T: Ord> Consumer for IntoConsumer<T> {
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
}

impl<T: Ord> crate::IntoConsumer for BTreeSet<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumer<T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumer(self)
    }
}

/// The consumer of the [`IntoConsumer`] impl of `&mut BTreeSet<T>`; it inserts consumed items to the [`BTreeSet`].
///
/// ```
/// use ufotofu::prelude::*;
/// use std::collections::BTreeSet;
/// # pollster::block_on(async{
/// let mut set = BTreeSet::new();
/// let mut c = (&mut set).into_consumer();
///
/// c.consume(1).await?;
/// c.consume(4).await?;
/// c.consume(2).await?;
///
/// let collected: Vec<_> = set.into_iter().collect();
/// assert_eq!(collected, vec![1, 2, 4]);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::btree_set::IntoProducerRef] type.
#[derive(Debug)]

pub struct IntoConsumerMut<'a, T>(&'a mut BTreeSet<T>);

impl<'a, T: Ord> Consumer for IntoConsumerMut<'a, T> {
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
}

impl<'a, T: Ord> crate::IntoConsumer for &'a mut BTreeSet<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumerMut<'a, T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumerMut(self)
    }
}
