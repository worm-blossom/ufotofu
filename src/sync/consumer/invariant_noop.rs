use core::mem::MaybeUninit;

use wrapper::Wrapper;

use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

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
///   - `consumer_slots`
///   - `did_consume`
///   - `bulk_consume`
/// - Must not call any of the prior functions after any of them had returned
///   an error.
/// - Must not call `did_consume` for slots that had not been exposed by
#[derive(Debug, Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub struct Invariant<C> {
    /// An implementer of the `Consumer` traits.
    inner: C,
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

    fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.inner.consume(item)
    }

    fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        self.inner.close(final_val)
    }
}

impl<C, T, F, E> BufferedConsumer for Invariant<C>
where
    C: BufferedConsumer<Item = T, Final = F, Error = E>,
{
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush()
    }
}

impl<C, T, F, E> BulkConsumer for Invariant<C>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn consumer_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        self.inner.consumer_slots()
    }

    unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.inner.did_consume(amount)
    }
}
