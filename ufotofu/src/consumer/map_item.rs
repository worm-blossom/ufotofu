use core::marker::PhantomData;

use crate::{BufferedConsumer, Consumer};

/// A `Consumer` adaptor that wraps any consumer and maps the items it receives with a function.
#[derive(Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub struct MapItem<C, F, B> {
    inner: C,
    fun: F,
    phantom: PhantomData<B>,
}

impl<C: core::fmt::Debug, F, B> core::fmt::Debug for MapItem<C, F, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MapItem")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<C, F, B> MapItem<C, F, B> {
    /// Returns a consumer that behaves like the wrapped consumer except it passes all received items through a function.
    pub fn new(inner: C, fun: F) -> Self {
        MapItem {
            inner,
            fun,
            phantom: PhantomData,
        }
    }

    /// Consumes `self` and returns the wrapped consumer.
    pub fn into_inner(self) -> C {
        self.inner
    }
}

impl<B, C, F> Consumer for MapItem<C, F, B>
where
    C: Consumer,
    F: FnMut(B) -> C::Item,
{
    type Item = B;
    type Final = C::Final;
    type Error = C::Error;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.inner.consume((self.fun)(item)).await
    }

    async fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        self.inner.close(final_val).await
    }
}

impl<B, C, F> BufferedConsumer for MapItem<C, F, B>
where
    C: BufferedConsumer,
    F: FnMut(B) -> C::Item,
{
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush().await
    }
}
