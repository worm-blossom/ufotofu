use either::Either::{self, Left, Right};

use crate::{BufferedProducer, BulkProducer, Producer};

/// A `Producer` adaptor that maps the final item emitted by an inner `Producer` with a function.
#[derive(Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub struct MapFinal<P, F> {
    inner: P,
    fun: Option<F>,
}

impl<P: core::fmt::Debug, F> core::fmt::Debug for MapFinal<P, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MapFinal")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<P, F> MapFinal<P, F> {
    /// Returns a producer that behaves like the wrapped producer except it passes its final item through a function.
    pub fn new(inner: P, fun: F) -> Self {
        MapFinal {
            inner,
            fun: Some(fun),
        }
    }

    /// Consumes `self` and returns the wrapped producer.
    pub fn into_inner(self) -> P {
        self.inner
    }
}

impl<B, P: Producer, F: FnOnce(P::Final) -> B> Producer for MapFinal<P, F> {
    type Item = P::Item;
    type Final = B;
    type Error = P::Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        match self.inner.produce().await? {
            Left(item) => Ok(Left(item)),
            Right(fin) => {
                Ok(Right((self.fun.take().expect(
                    "Must not produce items after the final item has been emitted",
                ))(fin)))
            }
        }
    }
}

impl<B, P: BufferedProducer, F: FnOnce(P::Final) -> B> BufferedProducer for MapFinal<P, F> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.inner.slurp().await
    }
}

impl<B, P: BulkProducer, F: FnOnce(P::Final) -> B> BulkProducer for MapFinal<P, F> {
    async fn expose_items<'a>(
        &'a mut self,
    ) -> Result<Either<&'a [Self::Item], Self::Final>, Self::Error>
    where
        Self::Item: 'a,
    {
        match self.inner.expose_items().await? {
            Left(items) => Ok(Left(items)),
            Right(fin) => {
                Ok(Right((self.fun.take().expect(
                    "Must not produce items after the final item has been emitted",
                ))(fin)))
            }
        }
    }

    async fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.inner.consider_produced(amount).await
    }
}
