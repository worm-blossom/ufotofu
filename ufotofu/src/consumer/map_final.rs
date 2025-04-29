use core::marker::PhantomData;

use crate::{BufferedConsumer, BulkConsumer, Consumer};

/// A `Consumer` adaptor that maps the final items it receives with a function before passing it to the inner consumer.
#[derive(Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub struct MapFinal<C, F, B> {
    inner: C,
    fun: Option<F>,
    phantom: PhantomData<B>,
}

impl<C: core::fmt::Debug, F, B> core::fmt::Debug for MapFinal<C, F, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MapFinal")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<C, F, B> MapFinal<C, F, B> {
    /// Returns a Consumer that forwards to the wrapped Consumer but passes its final item through a function.
    pub fn new(inner: C, fun: F) -> Self {
        MapFinal {
            inner,
            fun: Some(fun),
            phantom: PhantomData,
        }
    }

    /// Consumes `self` and returns the wrapped Consumer.
    pub fn into_inner(self) -> C {
        self.inner
    }
}

impl<B, C: Consumer, F: FnOnce(B) -> C::Final> Consumer for MapFinal<C, F, B> {
    type Item = C::Item;
    type Final = B;
    type Error = C::Error;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.inner.consume(item).await
    }

    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error> {
        self.inner
            .close((self
                .fun
                .take()
                .expect("Must not call close multiple times"))(
                fin
            ))
            .await
    }
}

impl<B, C: BufferedConsumer, F: FnOnce(B) -> C::Final> BufferedConsumer for MapFinal<C, F, B> {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush().await
    }
}

impl<B, C: BulkConsumer, F: FnOnce(B) -> C::Final> BulkConsumer for MapFinal<C, F, B> {
    async fn expose_slots<'a>(&'a mut self) -> Result<&'a mut [Self::Item], Self::Error>
    where
        Self::Item: 'a,
    {
        self.inner.expose_slots().await
    }

    async fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.inner.consume_slots(amount).await
    }
}
