//! Consumer functionality for [`Vec`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoConsumer`] impl for `Vec<T>`,
//! - an [`IntoConsumer`] impl for `&mut Vec<T>`.
//!
//! <br/>Counterpart: the [`producer::compat::vec`] module.

use core::fmt::Debug;

use alloc::vec::Vec;

use crate::prelude::*;

/// The consumer of the [`IntoConsumer`] impl of `Vec<T>`; it appends consumed data to the [`Vec`].
///
/// Use the [`Into`] impl to recover the vec when you are done consuming items.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut c = vec![].into_consumer();
///
/// c.consume(1).await?;
/// c.consume(2).await?;
/// c.consume(4).await?;
///
/// let vec: Vec<_> = c.into();
/// assert_eq!(vec, vec![1, 2, 4]);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::vec::IntoProducer] type.
#[derive(Debug)]

pub struct IntoConsumer<T>(Vec<T>);

impl<T> From<IntoConsumer<T>> for Vec<T> {
    fn from(value: IntoConsumer<T>) -> Self {
        value.0
    }
}

impl<T> Consumer for IntoConsumer<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;

    /// Appends the item to the vec.
    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.0.push(item);
        Ok(())
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!();
    }
}

impl<T: Clone> BulkConsumer for IntoConsumer<T> {
    async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
        debug_assert_ne!(
            buf.len(),
            0,
            "Must not call bulk_consume with an empty buffer."
        );

        self.0.extend_from_slice(buf);

        Ok(buf.len())
    }
}

impl<T> crate::IntoConsumer for Vec<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumer<T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumer(self)
    }
}

/// The consumer of the [`IntoConsumer`] impl of `&mut Vec<T>`; it appends consumed data to the [`Vec`].
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut v = vec![];
/// let mut c = (&mut v).into_consumer();
///
/// c.consume(1).await?;
/// c.consume(2).await?;
/// c.consume(4).await?;
///
/// assert_eq!(v, vec![1, 2, 4]);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::vec::IntoProducerMut] type.
#[derive(Debug)]

pub struct IntoConsumerMut<'a, T>(&'a mut Vec<T>);

impl<'a, T> Consumer for IntoConsumerMut<'a, T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;

    /// Appends the item to the vec.
    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.0.push(item);
        Ok(())
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!();
    }
}

impl<'a, T: Clone> BulkConsumer for IntoConsumerMut<'a, T> {
    async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
        debug_assert_ne!(
            buf.len(),
            0,
            "Must not call bulk_consume with an empty buffer."
        );

        self.0.extend_from_slice(buf);

        Ok(buf.len())
    }
}

impl<'a, T> crate::IntoConsumer for &'a mut Vec<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumerMut<'a, T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumerMut(self)
    }
}
