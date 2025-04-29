use crate::{BufferedConsumer, BulkConsumer, Consumer};

/// A `Consumer` adaptor that limits the number of regular items accepted by the Consumer.
#[derive(Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd, Debug)]
pub struct Limit<C> {
    inner: C,
    remaining: usize,
}

impl<C> Limit<C> {
    /// Returns a Consumer that emits an error after consuming too many regular items.
    ///
    /// ```rust
    /// use ufotofu::consumer::{Limit, IntoVec};
    /// use ufotofu::Consumer;
    /// use either::Either::*;
    ///
    /// let mut c = Limit::new(IntoVec::new(), 2);
    /// pollster::block_on(async {
    ///     assert_eq!(c.consume(0).await, Ok(()));
    ///     assert_eq!(c.consume(1).await, Ok(()));
    ///     assert_eq!(c.consume(2).await, Err(None));
    /// });
    /// ```
    pub fn new(inner: C, limit: usize) -> Self {
        Limit {
            inner,
            remaining: limit,
        }
    }

    /// Consumes `self` and returns the wrapped Consumer.
    pub fn into_inner(self) -> C {
        self.inner
    }
}

impl<C: Consumer> Consumer for Limit<C> {
    type Item = C::Item;
    type Final = C::Final;
    /// `None` if the limit was reached, `Some` if the wrapped Consumer emitted its error before the limit was reached.
    type Error = Option<C::Error>;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        match self.remaining.checked_sub(1) {
            None => Err(None),
            Some(decremented) => {
                self.remaining = decremented;
                self.inner.consume(item).await.map_err(Some)
            }
        }
    }

    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error> {
        self.inner.close(fin).await.map_err(Some)
    }
}

impl<C: BufferedConsumer> BufferedConsumer for Limit<C> {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush().await.map_err(Some)
    }
}

impl<C: BulkConsumer> BulkConsumer for Limit<C> {
    async fn expose_slots<'a>(&'a mut self) -> Result<&'a mut [Self::Item], Self::Error>
    where
        Self::Item: 'a,
    {
        if self.remaining == 0 {
            Err(None)
        } else {
            let slots = self.inner.expose_slots().await?;
            let len = slots.len();
            Ok(&mut slots[..core::cmp::min(self.remaining, len)])
        }
    }

    async fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.remaining -= amount;
        self.inner.consume_slots(amount).await.map_err(Some)
    }
}
