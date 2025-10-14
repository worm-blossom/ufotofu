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
#[derive(Debug, Clone)]

pub struct IntoConsumer<T>(Vec<T>, usize);
// The usize is the number of items consumed so far. For bulk consumption, we resize the Vec with default values to offer a slice, but those default values are then overwritten by further consumption.

impl<T> From<IntoConsumer<T>> for Vec<T> {
    fn from(value: IntoConsumer<T>) -> Self {
        value.0
    }
}

impl<T> IntoConsumer<T> {
    /// Exposes all items consumed so far as a slice.
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
    /// assert_eq!(c.as_slice(), [1, 2, 4].as_slice());
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    pub fn as_slice(&self) -> &[T] {
        &self.0[..self.1]
    }

    /// Exposes all items consumed so far as a mutable slice.
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
    /// assert_eq!(c.as_mut_slice(), [1, 2, 4].as_mut_slice());
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.0[..self.1]
    }

    /// Ensures that the next call to `self.expose_slots` will expose at least the requested number of slots.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut c = Vec::<u32>::new().into_consumer();
    ///
    /// c.prepare_slots(17);
    /// c.expose_slots(async |slots| {
    ///     assert!(slots.len() >= 17);
    ///     (0, ())
    /// }).await?;
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    pub fn prepare_slots(&mut self, amount: usize)
    where
        T: Default,
    {
        let old_len = self.0.len();
        self.0.resize_with(old_len + amount, Default::default);
    }
}

impl<T> Consumer for IntoConsumer<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;

    /// Appends the item to the vec.
    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.0.push(item);
        self.1 += 1;
        Ok(())
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!();
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T: Default> BulkConsumer for IntoConsumer<T> {
    async fn expose_slots<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R),
    {
        let len = self.0.len() - self.1;

        if len == 0 {
            let new_len = self.1 * 2 + 1;
            self.0.resize_with(new_len, Default::default);
        }

        let (amount, ret) = f(&mut self.0[self.1..]).await;
        self.1 += amount;
        Ok(ret)
    }
}

impl<T> crate::IntoConsumer for Vec<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumer<T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumer(self, 0)
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
/// <br/>Counterpart: the [producer::compat::vec::IntoProducerRef] type.
#[derive(Debug)]

pub struct IntoConsumerMut<'a, T>(&'a mut Vec<T>, usize);

impl<'a, T> Consumer for IntoConsumerMut<'a, T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;

    /// Appends the item to the vec.
    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.0.push(item);
        self.1 += 1;
        Ok(())
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!();
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a, T: Default> BulkConsumer for IntoConsumerMut<'a, T> {
    async fn expose_slots<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R),
    {
        let len = self.0.len() - self.1;

        if len == 0 {
            let new_len = self.1 * 2 + 1;
            self.0.resize_with(new_len, Default::default);
        }

        let (amount, ret) = f(&mut self.0[self.1..]).await;
        self.1 += amount;
        Ok(ret)
    }
}

impl<'a, T> crate::IntoConsumer for &'a mut Vec<T> {
    type Item = T;
    type Final = Infallible;
    type Error = Infallible;
    type IntoConsumer = IntoConsumerMut<'a, T>;

    fn into_consumer(self) -> Self::IntoConsumer {
        IntoConsumerMut(self, 0)
    }
}
