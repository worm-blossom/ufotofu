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
    /// The status of the consumer.
    active: bool,
    /// The number of available slots exposed by the `consumer_slots` method.
    exposed_slots: usize,
}

impl<C> Invariant<C> {
    /// Returns a new `Invariant` instance with `active` set to `true` and
    /// `exposed_slots` set to `0`.
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::sync::consumer::Cursor;

    // Panic conditions:
    //
    // - `consume()` must not be called after `close()` or error
    // - `close()` must not be called after `close()` or error
    // - `flush()` must not be called after `close()` or error
    // - `consumer_slots()` must not be called after `close()` or error
    // - `did_consume()` must not be called after `close()` or error
    // - `bulk_consume()` must not be called after `close()` or error
    // - `did_consume(amount)` must not be called with `amount` greater than available slots

    // In each of the following tests, the final function call should panic.

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_consume_after_close() {
        let mut buf = [0; 1];

        let mut cursor = Cursor::new(&mut buf);
        let _ = cursor.close(());
        let _ = cursor.consume(7);
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_close_after_close() {
        let mut buf = [0; 1];

        let mut cursor = Cursor::new(&mut buf);
        let _ = cursor.close(());
        let _ = cursor.close(());
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_flush_after_close() {
        let mut buf = [0; 1];

        let mut cursor = Cursor::new(&mut buf);
        let _ = cursor.close(());
        let _ = cursor.flush();
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_consumer_slots_after_close() {
        let mut buf = [0; 1];

        let mut cursor = Cursor::new(&mut buf);
        let _ = cursor.close(());
        let _ = cursor.consumer_slots();
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_did_consume_after_close() {
        let mut buf = [0; 8];

        let mut cursor = Cursor::new(&mut buf);
        let _ = cursor.close(());

        unsafe {
            let _ = cursor.did_consume(7);
        }
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_bulk_consume_after_close() {
        let mut buf = [0; 8];

        let mut cursor = Cursor::new(&mut buf);
        let _ = cursor.close(());
        let _ = cursor.bulk_consume(b"ufo");
    }

    #[test]
    #[should_panic(
        expected = "may not call `did_consume` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_consume_with_amount_greater_than_available_slots() {
        let mut buf = [0; 8];

        let mut cursor = Cursor::new(&mut buf);

        unsafe {
            let _ = cursor.did_consume(21);
        }
    }
}
