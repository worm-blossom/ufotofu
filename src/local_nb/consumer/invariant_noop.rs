use core::mem::MaybeUninit;

use wrapper::Wrapper;

use crate::local_nb::{BufferedConsumer, BulkConsumer, Consumer};

/// A `Consumer` wrapper that panics when callers violate API contracts such
/// as halting interaction after an error.
///
/// This wrapper only performs the checks when testing code (more specifically,
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
/// - Must not call any of the following functions after `close` had been called:
///   - `consume`
///   - `close`
///   - `flush`
///   - slots
///   - `consume_slots`
///   - `bulk_consume`
/// - Must not call any of the prior functions after any of them had returned
///   an error.
/// - Must not call `consume_slots` for slots that had not been exposed by
#[derive(Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub struct Invariant<C> {
    /// An implementer of the `Consumer` traits.
    inner: C,
}

impl<C: core::fmt::Debug> core::fmt::Debug for Invariant<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<C> Invariant<C> {
    /// Return a `Consumer` that behaves exactly like the wrapped `Consumer`
    /// `inner`.
    #[allow(dead_code)]
    pub fn new(inner: C) -> Self {
        Invariant { inner }
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

impl<C> Wrapper<C> for Invariant<C> {
    fn into_inner(self) -> C {
        self.inner
    }
}

impl<C, T, F, E> Consumer for Invariant<C>
where
    C: Consumer<Item = T, Final = F, Error = E>,
{
    type Item = T;
    type Final = F;
    type Error = E;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.inner.consume(item).await
    }

    async fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        self.inner.close(final_val).await
    }
}

impl<C, T, F, E> BufferedConsumer for Invariant<C>
where
    C: BufferedConsumer<Item = T, Final = F, Error = E>,
{
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush().await
    }
}

impl<C, T, F, E> BulkConsumer for Invariant<C>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    async fn expose_slots<'a>(
        &'a mut self,
    ) -> Result<&'a mut [MaybeUninit<Self::Item>], Self::Error>
    where
        T: 'a,
    {
        self.inner.expose_slots().await
    }

    async unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.inner.consume_slots(amount).await
    }
}
