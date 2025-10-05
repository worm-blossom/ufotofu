//! Consumer functionality for [`[T; N]`](core::array).
//!
//! Specifically, the module provides
//!
//! - an [`IntoConsumer`] impl for `[T; N]`,
//! - an [`IntoConsumer`] impl for `&mut [T; N]`.
//!
//! <br/>Counterpart: the [`producer::compat::array`] module.

use core::fmt::Debug;

use crate::prelude::*;

/// The consumer of the [`IntoConsumer`] impl of `[T; N]`; it sequentially overwrites the array with consumed data.
///
/// Use the [`Into`] impl to recover the array when you are done consuming items.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut c = [0, 0, 0].into_consumer();
///
/// c.consume(1).await?;
/// c.consume(2).await?;
/// c.consume(4).await?;
/// // Another `consume` call would return `Err(())`.
///
/// let arr: [_; 3] = c.into();
/// assert_eq!(arr, [1, 2, 4]);
/// # Result::<(), ()>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::array::IntoProducer] type.
#[derive(Debug)]

pub struct IntoConsumer<T, const N: usize>([T; N], usize);

impl<T, const N: usize> From<IntoConsumer<T, N>> for [T; N] {
    fn from(value: IntoConsumer<T, N>) -> Self {
        value.0
    }
}

impl<T, const N: usize> Consumer for IntoConsumer<T, N> {
    type Item = T;
    type Final = Infallible;
    type Error = ();

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        if self.1 < N {
            self.0[self.1] = item;
            self.1 += 1;
            Ok(())
        } else {
            Err(())
        }
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!();
    }
}

impl<T, const N: usize> BulkConsumer for IntoConsumer<T, N> {
    async fn expose_slots<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R),
    {
        let len = N - self.1;

        if len == 0 {
            Err(())
        } else {
            let (amount, ret) = f(&mut self.0[self.1..]).await;
            self.1 += amount;
            Ok(ret)
        }
    }
}

impl<T, const N: usize> crate::IntoConsumer for [T; N] {
    type Item = T;
    type Final = Infallible;
    type Error = ();
    type IntoConsumer = IntoConsumer<T, N>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumer(self, 0)
    }
}

/// The consumer of the [`IntoConsumer`] impl of `&mut [T; N]`; it sequentially overwrites the array with consumed data.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut arr = [0, 0, 0];
/// let mut c = (&mut arr).into_consumer();
///
/// c.consume(1).await?;
/// c.consume(2).await?;
/// c.consume(4).await?;
/// // Another `consume` call would return `Err(())`.
///
/// assert_eq!(arr, [1, 2, 4]);
/// # Result::<(), ()>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::compat::array::IntoProducerRef] type.
#[derive(Debug)]

pub struct IntoConsumerMut<'a, T, const N: usize>(&'a mut [T; N], usize);

impl<'a, T, const N: usize> Consumer for IntoConsumerMut<'a, T, N> {
    type Item = T;
    type Final = Infallible;
    type Error = ();

    /// Appends the item to the array, returning an error after overwriting the final slot.
    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        if self.1 < N {
            self.0[self.1] = item;
            self.1 += 1;
            Ok(())
        } else {
            Err(())
        }
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!();
    }
}

impl<'a, T, const N: usize> BulkConsumer for IntoConsumerMut<'a, T, N> {
    async fn expose_slots<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R),
    {
        let len = N - self.1;

        if len == 0 {
            Err(())
        } else {
            let (amount, ret) = f(&mut self.0[self.1..]).await;
            self.1 += amount;
            Ok(ret)
        }
    }
}

impl<'a, T, const N: usize> crate::IntoConsumer for &'a mut [T; N] {
    type Item = T;
    type Final = Infallible;
    type Error = ();
    type IntoConsumer = IntoConsumerMut<'a, T, N>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumerMut(self, 0)
    }
}
