use either::Either;

use crate::{BufferedProducer, BulkProducer, Producer};

/// A `Producer` adaptor that maps the error of an inner `Producer` with a function.
#[derive(Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub struct MapError<P, F> {
    inner: P,
    fun: Option<F>,
}

impl<P: core::fmt::Debug, F> core::fmt::Debug for MapError<P, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MapError")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<P, F> MapError<P, F> {
    /// Returns a producer that behaves like the wrapped producer except it passes its error through a function.
    pub fn new(inner: P, fun: F) -> Self {
        MapError {
            inner,
            fun: Some(fun),
        }
    }

    /// Consumes `self` and returns the wrapped producer.
    pub fn into_inner(self) -> P {
        self.inner
    }
}

impl<B, P: Producer, F: FnOnce(P::Error) -> B> Producer for MapError<P, F> {
    type Item = P::Item;
    type Final = P::Final;
    type Error = B;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.inner.produce().await.map_err(
            self.fun
                .take()
                .expect("Must not use a consumer after it had emitted an error"),
        )
    }
}

impl<B, P: BufferedProducer, F: FnOnce(P::Error) -> B> BufferedProducer for MapError<P, F> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.inner.slurp().await.map_err(
            self.fun
                .take()
                .expect("Must not use a consumer after it had emitted an error"),
        )
    }
}

impl<B, P: BulkProducer, F: FnOnce(P::Error) -> B> BulkProducer for MapError<P, F> {
    async fn expose_items<'a>(
        &'a mut self,
    ) -> Result<Either<&'a [Self::Item], Self::Final>, Self::Error>
    where
        Self::Item: 'a,
    {
        self.inner.expose_items().await.map_err(
            self.fun
                .take()
                .expect("Must not use a consumer after it had emitted an error"),
        )
    }

    async fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.inner.consider_produced(amount).await.map_err(
            self.fun
                .take()
                .expect("Must not use a consumer after it had emitted an error"),
        )
    }
}
