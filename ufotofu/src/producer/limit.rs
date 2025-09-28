use either::Either::{self, Left, Right};

use crate::{BufferedProducer, BulkProducer, Producer};

/// A `Producer` adaptor that limits the number of items emitted by the inner producer.
#[derive(Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd, Debug)]
pub struct Limit<P> {
    inner: P,
    remaining: usize,
}

impl<P> Limit<P> {
    /// Returns a producer that limits the number of items emitted by the inner producer.
    ///
    /// ```rust
    /// use ufotofu::producer::{Limit, FromBoxedSlice};
    /// use ufotofu::Producer;
    /// use either::Either::*;
    ///
    /// let mut p1 = Limit::new(FromBoxedSlice::from_vec(vec![0, 1, 2]), 2);
    /// pollster::block_on(async {
    ///     assert_eq!(p1.produce().await, Ok(Left(0)));
    ///     assert_eq!(p1.produce().await, Ok(Left(1)));
    ///     assert_eq!(p1.produce().await, Ok(Right(None)));
    /// });
    ///
    /// let mut p2 = Limit::new(FromBoxedSlice::from_vec(vec![0, 1, 2]), 4);
    /// pollster::block_on(async {
    ///     assert_eq!(p2.produce().await, Ok(Left(0)));
    ///     assert_eq!(p2.produce().await, Ok(Left(1)));
    ///     assert_eq!(p2.produce().await, Ok(Left(2)));
    ///     assert_eq!(p2.produce().await, Ok(Right(Some(()))));
    /// });
    /// ```
    pub fn new(inner: P, limit: usize) -> Self {
        Limit {
            inner,
            remaining: limit,
        }
    }

    /// Consumes `self` and returns the wrapped producer.
    pub fn into_inner(self) -> P {
        self.inner
    }
}

impl<P: Producer> Producer for Limit<P> {
    type Item = P::Item;
    /// `None` if the limit was reached, `Some` if the wrapped producer emitted its final value before the limit was reached.
    type Final = Option<P::Final>;
    type Error = P::Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        match self.remaining.checked_sub(1) {
            None => Ok(Right(None)),
            Some(decremented) => {
                self.remaining = decremented;
                match self.inner.produce().await? {
                    Left(item) => Ok(Left(item)),
                    Right(fin) => Ok(Right(Some(fin))),
                }
            }
        }
    }
}

impl<P: BufferedProducer> BufferedProducer for Limit<P> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.inner.slurp().await
    }
}

impl<P: BulkProducer> BulkProducer for Limit<P> {
    async fn expose_items<'a>(
        &'a mut self,
    ) -> Result<Either<&'a [Self::Item], Self::Final>, Self::Error>
    where
        Self::Item: 'a,
    {
        if self.remaining == 0 {
            Ok(Right(None))
        } else {
            match self.inner.expose_items().await? {
                Left(items) => Ok(Left(&items[..core::cmp::min(self.remaining, items.len())])),
                Right(fin) => Ok(Right(Some(fin))),
            }
        }
    }

    async fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.remaining -= amount;
        self.inner.consider_produced(amount).await
    }
}
