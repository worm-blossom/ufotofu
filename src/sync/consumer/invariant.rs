use core::mem::MaybeUninit;

use wrapper::Wrapper;

use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

/// A `Consumer` wrapper with additional fields, used to validate that
/// invariant contracts are upheld.
#[derive(Clone, Copy)]
pub struct Invariant<I> {
    /// An implementer of the `Consumer` traits.
    inner: I,
    /// The status of the consumer.
    active: bool,
    /// The number of available slots exposed by the `consumer_slots` method.
    exposed_slots: usize,
}

impl<I> Invariant<I> {
    /// Returns a new `Invariant` instance with `active` set to `true` and
    /// `exposed_slots` set to `0`.
    pub fn new(inner: I) -> Self {
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

impl<I> AsRef<I> for Invariant<I> {
    fn as_ref(&self) -> &I {
        &self.inner
    }
}

impl<I> AsMut<I> for Invariant<I> {
    fn as_mut(&mut self) -> &mut I {
        &mut self.inner
    }
}

impl<I> Wrapper<I> for Invariant<I> {
    fn into_inner(self) -> I {
        self.inner
    }
}

impl<I, T, F, E> Consumer for Invariant<I>
where
    I: BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
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
            self.exposed_slots = 0;
        })
    }

    fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        self.check_inactive();
        self.active = false;

        self.inner.close(final_val)
    }
}

impl<I, T, F, E> BufferedConsumer for Invariant<I>
where
    I: BufferedConsumer<Item = T, Final = F, Error = E>
        + BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.check_inactive();

        self.inner.flush().inspect_err(|_| {
            self.active = false;
            self.exposed_slots = 0;
        })
    }
}

impl<I, T, F, E> BulkConsumer for Invariant<I>
where
    I: BulkConsumer<Item = T, Final = F, Error = E>,
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
                self.exposed_slots = 0;
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

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_consume_after_close() {
        let mut buf = [0; 1];

        let mut cursor = Cursor::new(&mut buf);
        let _ = cursor.close(());

        // This call should panic.
        let _ = cursor.consume(7);
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_close_after_close() {
        let mut buf = [0; 1];

        let mut cursor = Cursor::new(&mut buf);
        let _ = cursor.close(());

        // This call should panic.
        let _ = cursor.close(());
    }
}
