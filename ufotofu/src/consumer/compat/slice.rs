//! Consumer functionality for [`[T]`](core::slice).
//!
//! Specifically, the module provides
//!
//! - an [`IntoConsumer`] impl for `&mut [T]`,
//! - an [`IntoConsumer`] impl for `Box<[T]>`, and
//! - an [`IntoConsumer`] impl for `&mut Box<[T]>`.
//!
//! <br/>Counterpart: the [`consumer::compat::slice`] module.

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

use crate::prelude::*;

/// The  of the [`IntoConsumer`] impl of `&mut [T]`.
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
/// <br/>Counterpart: [producer::compat::slice::IntoProducerRef].
pub struct IntoConsumerMut<'s, T>(&'s mut [T], usize);

impl<'s, T> Consumer for IntoConsumerMut<'s, T> {
    type Item = T;
    type Final = Infallible;
    type Error = ();

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        if self.1 < self.0.len() {
            self.0[self.1] = item;
            self.1 += 1;
            Ok(())
        } else {
            Err(())
        }
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!()
    }
}

impl<'a, T> BulkConsumer for IntoConsumerMut<'a, T> {
    async fn expose_slots<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R),
    {
        let len = self.0.len() - self.1;

        if len == 0 {
            Err(())
        } else {
            let (amount, ret) = f(&mut self.0[self.1..]).await;
            self.1 += amount;
            Ok(ret)
        }
    }
}

impl<'s, T> crate::IntoConsumer for &'s mut [T] {
    type Item = T;
    type Final = Infallible;
    type Error = ();
    type IntoConsumer = IntoConsumerMut<'s, T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumerMut(self, 0)
    }
}

/// The consumer of the [`IntoConsumer`] impl of `Box<[T]>`.
///
/// Use the [`Into`] impl to recover the box when you are done consuming items.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut c = vec![0, 0, 0].into_boxed_slice().into_consumer();
///
/// c.consume(1).await?;
/// c.consume(2).await?;
/// c.consume(4).await?;
/// // Another `consume` call would return `Err(())`.
///
/// let boxed_slice: Box<[_]> = c.into();
/// assert_eq!(boxed_slice, vec![1, 2, 4].into_boxed_slice());
/// # Result::<(), ()>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [producer::compat::slice::IntoProducerBoxed].
#[cfg(feature = "alloc")]
pub struct IntoConsumerBoxed<T>(Box<[T]>, usize);

#[cfg(feature = "alloc")]
impl<T> From<IntoConsumerBoxed<T>> for Box<[T]> {
    fn from(value: IntoConsumerBoxed<T>) -> Self {
        value.0
    }
}

#[cfg(feature = "alloc")]
impl<T> Consumer for IntoConsumerBoxed<T> {
    type Item = T;
    type Final = Infallible;
    type Error = ();

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        if self.1 < self.0.len() {
            self.0[self.1] = item;
            self.1 += 1;
            Ok(())
        } else {
            Err(())
        }
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!()
    }
}

#[cfg(feature = "alloc")]
impl<T> BulkConsumer for IntoConsumerBoxed<T> {
    async fn expose_slots<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R),
    {
        let len = self.0.len() - self.1;

        if len == 0 {
            Err(())
        } else {
            let (amount, ret) = f(&mut self.0[self.1..]).await;
            self.1 += amount;
            Ok(ret)
        }
    }
}

#[cfg(feature = "alloc")]
impl<T> crate::IntoConsumer for Box<[T]> {
    type Item = T;
    type Final = Infallible;
    type Error = ();
    type IntoConsumer = IntoConsumerBoxed<T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumerBoxed(self, 0)
    }
}

/// The consumer of the [`IntoConsumer`] impl of `&mut Box<[T]>`.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut boxed_slice = vec![0, 0, 0].into_boxed_slice();
/// let mut c = (&mut boxed_slice).into_consumer();
///
/// c.consume(1).await?;
/// c.consume(2).await?;
/// c.consume(4).await?;
/// // Another `consume` call would return `Err(())`.
///
/// assert_eq!(boxed_slice, vec![1, 2, 4].into_boxed_slice());
/// # Result::<(), ()>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [producer::compat::slice::IntoProducerBoxedRef].
#[cfg(feature = "alloc")]
pub struct IntoConsumerBoxedMut<'s, T>(&'s mut Box<[T]>, usize);

#[cfg(feature = "alloc")]
impl<'s, T> Consumer for IntoConsumerBoxedMut<'s, T> {
    type Item = T;
    type Final = Infallible;
    type Error = ();

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        if self.1 < self.0.len() {
            self.0[self.1] = item;
            self.1 += 1;
            Ok(())
        } else {
            Err(())
        }
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!()
    }
}

#[cfg(feature = "alloc")]
impl<'s, T> BulkConsumer for IntoConsumerBoxedMut<'s, T> {
    async fn expose_slots<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R),
    {
        let len = self.0.len() - self.1;

        if len == 0 {
            Err(())
        } else {
            let (amount, ret) = f(&mut self.0[self.1..]).await;
            self.1 += amount;
            Ok(ret)
        }
    }
}

#[cfg(feature = "alloc")]
impl<'s, T> crate::IntoConsumer for &'s mut Box<[T]> {
    type Item = T;
    type Final = Infallible;
    type Error = ();
    type IntoConsumer = IntoConsumerBoxedMut<'s, T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumerBoxedMut(self, 0)
    }
}
