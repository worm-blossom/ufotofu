use crate::{BufferedConsumer, BulkConsumer, Consumer};

/// A `Consumer` adaptor that maps the errors emitted by its inner consumer with a function.
#[derive(Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub struct MapError<C, F> {
    inner: C,
    fun: Option<F>,
}

impl<C: core::fmt::Debug, F> core::fmt::Debug for MapError<C, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MapError")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<C, F> MapError<C, F> {
    /// Returns a Consumer that maps the error emitted by the inner consumer but otherwise behaves identically.
    pub fn new(inner: C, fun: F) -> Self {
        MapError {
            inner,
            fun: Some(fun),
        }
    }

    /// Consumes `self` and returns the wrapped Consumer.
    pub fn into_inner(self) -> C {
        self.inner
    }
}

impl<B, C: Consumer, F: FnOnce(C::Error) -> B> Consumer for MapError<C, F> {
    type Item = C::Item;
    type Final = C::Final;
    type Error = B;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.inner.consume(item).await.map_err(
            self.fun
                .take()
                .expect("Must not use a consumer after it had emitted an error"),
        )
    }

    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error> {
        self.inner.close(fin).await.map_err(
            self.fun
                .take()
                .expect("Must not use a consumer after it had emitted an error"),
        )
    }
}

impl<B, C: BufferedConsumer, F: FnOnce(C::Error) -> B> BufferedConsumer for MapError<C, F> {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush().await.map_err(
            self.fun
                .take()
                .expect("Must not use a consumer after it had emitted an error"),
        )
    }
}

impl<B, C: BulkConsumer, F: FnOnce(C::Error) -> B> BulkConsumer for MapError<C, F> {
    async fn expose_slots<'a>(&'a mut self) -> Result<&'a mut [Self::Item], Self::Error>
    where
        Self::Item: 'a,
    {
        self.inner.expose_slots().await.map_err(
            self.fun
                .take()
                .expect("Must not use a consumer after it had emitted an error"),
        )
    }

    async fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.inner.consume_slots(amount).await.map_err(
            self.fun
                .take()
                .expect("Must not use a consumer after it had emitted an error"),
        )
    }
}
