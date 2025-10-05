//! Consumer functionality for [`VecDeque`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoConsumer`] impl for `VecDeque<T>`,
//! - an [`IntoConsumer`] impl for `&mut VecDeque<T>`.
//!
//! <br/>Counterpart: the [`producer::compat::vec_deque`] module.

use core::fmt::Debug;

use std::collections::VecDeque;

use crate::prelude::*;

/// The consumer of the [`IntoConsumer`] impl of `VecDeque<T>`; it adds consumed data at the back of the [`VecDeque`].
///
/// Use the [`Into`] impl to recover the deque when you are done consuming items.
///
/// ```
/// use ufotofu::prelude::*;
/// use std::collections::VecDeque;
/// # pollster::block_on(async{
/// let mut c = VecDeque::new().into_consumer();
///
/// c.consume(1).await?;
/// c.consume(2).await?;
/// c.consume(4).await?;
///
/// let mut deque: VecDeque<_> = c.into();
/// let collected: Vec<_> = deque.into_iter().collect();
/// assert_eq!(collected, vec![1, 2, 4]);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::vec_deque::IntoProducer] type.
#[derive(Debug)]

pub struct IntoConsumer<T>(VecDeque<T>);

impl<T> From<IntoConsumer<T>> for VecDeque<T> {
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

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T> crate::IntoConsumer for VecDeque<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumer<T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumer(self)
    }
}

/// The consumer of the [`IntoConsumer`] impl of `&mut VecDeque<T>`; it adds consumed data at the back of the [`VecDeque`].
///
/// ```
/// use ufotofu::prelude::*;
/// use std::collections::VecDeque;
/// # pollster::block_on(async{
/// let mut deque = VecDeque::new();
/// let mut c = (&mut deque).into_consumer();
///
/// c.consume(1).await?;
/// c.consume(2).await?;
/// c.consume(4).await?;
///
/// let collected: Vec<_> = deque.into_iter().collect();
/// assert_eq!(collected, vec![1, 2, 4]);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::vec_deque::IntoProducerRef] type.
#[derive(Debug)]

pub struct IntoConsumerMut<'a, T>(&'a mut VecDeque<T>);

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

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a, T> crate::IntoConsumer for &'a mut VecDeque<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumerMut<'a, T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumerMut(self)
    }
}
