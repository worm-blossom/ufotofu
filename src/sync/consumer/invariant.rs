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
///   `consumer_slots` before.
#[derive(Debug, Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub struct Invariant<C> {
    /// An implementer of the `Consumer` traits.
    inner: C,
    /// The status of the consumer. `true` while the caller may call trait
    /// methods, `false` once that becomes disallowed (because a method returned
    /// an error, or because `close` was called).
    active: bool,
    /// The maximum `amount` that a caller may supply to `did_consume`.
    exposed_slots: usize,
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

impl<C, T, F, E> Consumer for Invariant<C>
where
    C: Consumer<Item = T, Final = F, Error = E>,
{
    type Item = T;
    type Final = F;
    type Error = E;

    fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.check_inactive();

        self.inner.consume(item).inspect_err(|_| {
            // Since `consume()` returned an error, we need to ensure
            // that any future call to trait methods will panic.
            self.active = false;
        })
    }

    fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        self.check_inactive();
        self.active = false;

        self.inner.close(final_val)
    }
}

impl<C, T, F, E> BufferedConsumer for Invariant<C>
where
    C: BufferedConsumer<Item = T, Final = F, Error = E>,
{
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.check_inactive();

        self.inner.flush().inspect_err(|_| {
            self.active = false;
        })
    }
}

impl<C, T, F, E> BulkConsumer for Invariant<C>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn consumer_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        self.check_inactive();

        self.inner
            .consumer_slots()
            .inspect(|slots| {
                self.exposed_slots = slots.len();
            })
            .inspect_err(|_| {
                self.active = false;
            })
    }

    unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.check_inactive();

        if amount > self.exposed_slots {
            panic!(
                "may not call `did_consume` with an amount exceeding the total number of exposed slots"
            );
        } else {
            self.exposed_slots -= amount;
        }

        // Proceed with the inner call to `did_consume` and return the result.
        self.inner.did_consume(amount)
    }
}
