use either::Either::{self, Left, Right};

use crate::{BufferedProducer, Producer};

/// A `Producer` adaptor that maps the items emitted by an inner `Producer` with a function.
#[derive(Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub struct MapItem<P, F> {
    inner: P,
    fun: F,
}

impl<P: core::fmt::Debug, F> core::fmt::Debug for MapItem<P, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MapItem")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<P, F> MapItem<P, F> {
    /// Returns a producer that behaves like the wrapped producer except it passes all emitted items through a function.
    pub fn new(inner: P, fun: F) -> Self {
        MapItem { inner, fun }
    }

    /// Consumes `self` and returns the wrapped producer.
    pub fn into_inner(self) -> P {
        self.inner
    }
}

impl<B, P: Producer, F: FnMut(P::Item) -> B> Producer for MapItem<P, F> {
    type Item = B;
    type Final = P::Final;
    type Error = P::Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        match self.inner.produce().await? {
            Left(item) => Ok(Left((self.fun)(item))),
            Right(fin) => Ok(Right(fin)),
        }
    }
}

impl<B, P: BufferedProducer, F: FnMut(P::Item) -> B> BufferedProducer for MapItem<P, F> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.inner.slurp().await
    }
}
