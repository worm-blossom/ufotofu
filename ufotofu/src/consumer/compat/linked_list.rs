//! Consumer functionality for [`LinkedList`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoConsumer`] impl for `LinkedList<T>`,
//! - an [`IntoConsumer`] impl for `&mut LinkedList<T>`.
//!
//! <br/>Counterpart: the [`producer::compat::linked_list`] module.

use core::fmt::Debug;

use std::collections::LinkedList;

use crate::prelude::*;

/// The consumer of the [`IntoConsumer`] impl of `LinkedList<T>`; it adds consumed data at the back of the [`LinkedList`].
///
/// Use the [`Into`] impl to recover the list when you are done consuming items.
///
/// ```
/// use ufotofu::prelude::*;
/// use std::collections::LinkedList;
/// # pollster::block_on(async{
/// let mut c = LinkedList::new().into_consumer();
///
/// c.consume(1).await?;
/// c.consume(2).await?;
/// c.consume(4).await?;
///
/// let mut list: LinkedList<_> = c.into();
/// let collected: Vec<_> = list.into_iter().collect();
/// assert_eq!(collected, vec![1, 2, 4]);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::linked_list::IntoProducer] type.
#[derive(Debug)]

pub struct IntoConsumer<T>(LinkedList<T>);

impl<T> From<IntoConsumer<T>> for LinkedList<T> {
    fn from(value: IntoConsumer<T>) -> Self {
        value.0
    }
}

impl<T> Consumer for IntoConsumer<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.0.push_back(item);
        Ok(())
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!();
    }
}

impl<T> crate::IntoConsumer for LinkedList<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumer<T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumer(self)
    }
}

/// The consumer of the [`IntoConsumer`] impl of `&mut LinkedList<T>`; it adds consumed data at the back of the [`LinkedList`].
///
/// ```
/// use ufotofu::prelude::*;
/// use std::collections::LinkedList;
/// # pollster::block_on(async{
/// let mut list = LinkedList::new();
/// let mut c = (&mut list).into_consumer();
///
/// c.consume(1).await?;
/// c.consume(2).await?;
/// c.consume(4).await?;
///
/// let collected: Vec<_> = list.into_iter().collect();
/// assert_eq!(collected, vec![1, 2, 4]);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::linked_list::IntoProducerMut] type.
#[derive(Debug)]

pub struct IntoConsumerMut<'a, T>(&'a mut LinkedList<T>);

impl<'a, T> Consumer for IntoConsumerMut<'a, T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.0.push_back(item);
        Ok(())
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!();
    }
}

impl<'a, T> crate::IntoConsumer for &'a mut LinkedList<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumerMut<'a, T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumerMut(self)
    }
}
