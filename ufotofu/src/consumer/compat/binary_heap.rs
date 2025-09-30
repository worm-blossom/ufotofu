//! Consumer functionality for [`BinaryHeap`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoConsumer`] impl for `BinaryHeap<T>`,
//! - an [`IntoConsumer`] impl for `&mut BinaryHeap<T>`.
//!
//! <br/>Counterpart: the [`producer::compat::binary_heap`] module.

use core::fmt::Debug;

use std::collections::BinaryHeap;

use crate::prelude::*;

/// The consumer of the [`IntoConsumer`] impl of `BinaryHeap<T>`; it adds consumed data to the [`BinaryHeap`].
///
/// Use the [`Into`] impl to recover the heap when you are done consuming items.
///
/// ```
/// use ufotofu::prelude::*;
/// use std::collections::BinaryHeap;
/// # pollster::block_on(async{
/// let mut c = BinaryHeap::new().into_consumer();
///
/// c.consume(1).await?;
/// c.consume(4).await?;
/// c.consume(2).await?;
///
/// let mut heap: BinaryHeap<_> = c.into();
/// assert_eq!(heap.pop(), Some(4));
/// assert_eq!(heap.pop(), Some(2));
/// assert_eq!(heap.pop(), Some(1));
/// assert_eq!(heap.pop(), None);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::binary_heap::IntoProducer] type.
#[derive(Debug)]

pub struct IntoConsumer<T>(BinaryHeap<T>);

impl<T> From<IntoConsumer<T>> for BinaryHeap<T> {
    fn from(value: IntoConsumer<T>) -> Self {
        value.0
    }
}

impl<T: Ord> Consumer for IntoConsumer<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.0.push(item);
        Ok(())
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!();
    }
}

impl<T: Ord> crate::IntoConsumer for BinaryHeap<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumer<T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumer(self)
    }
}

/// The consumer of the [`IntoConsumer`] impl of `&mut BinaryHeap<T>`; it adds consumed data to the [`BinaryHeap`].
///
/// ```
/// use ufotofu::prelude::*;
/// use std::collections::BinaryHeap;
/// # pollster::block_on(async{
/// let mut heap = BinaryHeap::new();
/// let mut c = (&mut heap).into_consumer();
///
/// c.consume(1).await?;
/// c.consume(4).await?;
/// c.consume(2).await?;
///
/// assert_eq!(heap.pop(), Some(4));
/// assert_eq!(heap.pop(), Some(2));
/// assert_eq!(heap.pop(), Some(1));
/// assert_eq!(heap.pop(), None);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::binary_heap::IntoProducerMut] type.
#[derive(Debug)]

pub struct IntoConsumerMut<'a, T>(&'a mut BinaryHeap<T>);

impl<'a, T: Ord> Consumer for IntoConsumerMut<'a, T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.0.push(item);
        Ok(())
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!();
    }
}

impl<'a, T: Ord> crate::IntoConsumer for &'a mut BinaryHeap<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumerMut<'a, T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumerMut(self)
    }
}
