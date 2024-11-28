use wrapper::Wrapper;

use crate::{BufferedConsumer, BulkConsumer, Consumer};

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
/// - Must not call trait methods of [`Consumer`], [`BufferedConsumer`], or [`BulkConsumer`] after [`close`](Consumer::close) has been called.
/// - Must not call any of the trait methods after any of them have returned
///   an error.
/// - Must not call [`consume_slots`](BulkConsumer::consume_slots) with an amount exceeding the number of available exposed slots.
#[derive(Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
#[cfg_attr(feature = "dev", derive(arbitrary::Arbitrary))]
pub struct Invariant<C> {
    /// An implementer of the `Consumer` traits.
    inner: C,
    /// The status of the consumer. `true` while the caller may call trait
    /// methods, `false` once that becomes disallowed (because a method returned
    /// an error, or because `close` was called).
    active: bool,
    /// The maximum `amount` that a caller may supply to `consume_slots`.
    exposed_slots: usize,
}

impl<C: core::fmt::Debug> core::fmt::Debug for Invariant<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<C> Invariant<C> {
    /// Return a `Consumer` that behaves exactly like the wrapped `Consumer`
    /// `inner`, except that - when running tests - it performs runtime
    /// validation of API invariants and panics if they are violated by a
    /// caller.
    pub fn new(inner: C) -> Self {
        Invariant {
            inner,
            active: true,
            exposed_slots: 0,
        }
    }

    /// Checks the state of the `active` field and panics if the value is
    /// `false`.
    fn check_inactive(&self) {
        if !self.active {
            panic!("may not call `Consumer` methods after the sequence has ended");
        }
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

impl<C> Consumer for Invariant<C>
where
    C: Consumer,
{
    type Item = C::Item;
    type Final = C::Final;
    type Error = C::Error;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.check_inactive();

        self.inner.consume(item).await.inspect_err(|_| {
            // Since `consume()` returned an error, we need to ensure
            // that any future call to trait methods will panic.
            self.active = false;
        })
    }

    async fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        self.check_inactive();
        self.active = false;

        self.inner.close(final_val).await
    }
}

impl<C> BufferedConsumer for Invariant<C>
where
    C: BufferedConsumer,
{
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.check_inactive();

        self.inner.flush().await.inspect_err(|_| {
            self.active = false;
        })
    }
}

impl<C> BulkConsumer for Invariant<C>
where
    C: BulkConsumer,
    C::Item: Copy,
{
    async fn expose_slots<'a>(&'a mut self) -> Result<&'a mut [Self::Item], Self::Error>
    where
        Self::Item: 'a,
    {
        self.check_inactive();

        self.inner
            .expose_slots()
            .await
            .inspect(|slots| {
                self.exposed_slots = slots.len();
            })
            .inspect_err(|_| {
                self.active = false;
            })
    }

    async fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.check_inactive();

        if amount > self.exposed_slots {
            panic!(
                "may not call `consume_slots` with an amount exceeding the total number of exposed slots"
            );
        } else {
            self.exposed_slots -= amount;
        }

        // Proceed with the inner call to `consume_slots` and return the result.
        self.inner.consume_slots(amount).await
    }
}
