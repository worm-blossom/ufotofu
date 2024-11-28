use crate::{BufferedConsumer, BulkConsumer, Consumer};

/// A `Consumer` wrapper that panics when callers violate API contracts such
/// as halting interaction after an error.
///
/// This wrapper only performs the checks while testing (more specifically,
/// when `#[cfg(test)]` applies). In production builds, the wrapper does
/// nothing at all and compiles away without any overhead.
///
/// All consumers implemented in this crate use this wrapper internally already.
/// We recommend to use this type for all custom consumers as well.
///
/// #### Invariants
///
/// The wrapper enforces the following invariants:
///
/// - Must not call trait methods of [`Consumer`], [`BufferedConsumer`], or [`BulkConsumer`] after [`close`](Consumer::close) has been called.
/// - Must not call any of the trait methods after any of them have returned
///   an error.
/// - Must not call [`consume_slots`](BulkConsumer::consume_slots) with an amount exceeding the number of available exposed slots.
#[derive(Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
#[cfg_attr(feature = "dev", derive(arbitrary::Arbitrary))]
pub struct Invariant<C> {
    inner: C,
}

impl<C: core::fmt::Debug> core::fmt::Debug for Invariant<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<C> Invariant<C> {
    /// Returns a consumer that behaves exactly like the wrapped consumer
    /// `inner`, except that - when running tests - it performs runtime
    /// validation of API invariants and panics if they are violated by a
    /// caller.
    pub fn new(inner: C) -> Self {
        Invariant { inner }
    }

    /// Consumes `self` and returns the wrapped consumer.
    pub fn into_inner(self) -> C {
        self.inner
    }
}

impl<C> AsRef<C> for Invariant<C> {
    fn as_ref(&self) -> &C {
        &self.inner
    }
}

impl<C> AsMut<C> for Invariant<C> {
    fn as_mut(&mut self) -> &mut C {
        &mut self.inner
    }
}

impl<C> Consumer for Invariant<C>
where
    C: Consumer,
{
    type Item = C::Item;
    type Final = C::Final;
    type Error = C::Error;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.inner.consume(item).await
    }

    async fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        self.inner.close(final_val).await
    }
}

impl<C> BufferedConsumer for Invariant<C>
where
    C: BufferedConsumer,
{
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush().await
    }
}

impl<C> BulkConsumer for Invariant<C>
where
    C: BulkConsumer,
{
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
