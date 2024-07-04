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
    #[allow(dead_code)]
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

impl<C, T, F, E> BufferedConsumer for Invariant<C>
where
    C: BufferedConsumer<Item = T, Final = F, Error = E>,
{
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.check_inactive();

        self.inner.flush().await.inspect_err(|_| {
            self.active = false;
        })
    }
}

impl<C, T, F, E> BulkConsumer for Invariant<C>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    async fn consumer_slots<'a>(
        &'a mut self,
    ) -> Result<&'a mut [MaybeUninit<Self::Item>], Self::Error>
    where
        T: 'a,
    {
        self.check_inactive();

        self.inner
            .consumer_slots()
            .await
            .inspect(|slots| {
                self.exposed_slots = slots.len();
            })
            .inspect_err(|_| {
                self.active = false;
            })
    }

    async unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.check_inactive();

        if amount > self.exposed_slots {
            panic!(
                "may not call `did_consume` with an amount exceeding the total number of exposed slots"
            );
        } else {
            self.exposed_slots -= amount;
        }

        // Proceed with the inner call to `did_consume` and return the result.
        self.inner.did_consume(amount).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::local_nb::consumer::slice_consumer::SliceConsumerFullError;
    use crate::local_nb::consumer::{IntoVec, SliceConsumer};

    #[test]
    fn accepts_valid_did_consume_amount() {
        smol::block_on(async {
            // Create a slice consumer that exposes four slots.
            let mut buf = [0; 4];
            let mut slice_consumer = SliceConsumer::new(&mut buf);

            // Copy data to three of the available slots and call `did_consume`.
            let data = b"ufo";
            let slots = slice_consumer.consumer_slots().await.unwrap();
            MaybeUninit::copy_from_slice(&mut slots[0..3], &data[0..3]);
            unsafe {
                assert!(slice_consumer.did_consume(3).await.is_ok());
            }
        })
    }

    #[test]
    #[should_panic(
        expected = "may not call `did_consume` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_second_did_consume_with_amount_greater_than_available_slots() {
        smol::block_on(async {
            // Create a slice consumer that exposes four slots.
            let mut buf = [0; 4];
            let mut slice_consumer = SliceConsumer::new(&mut buf);

            // Copy data to three of the available slots and call `did_consume`.
            let data = b"ufo";
            let slots = slice_consumer.consumer_slots().await.unwrap();
            MaybeUninit::copy_from_slice(&mut slots[0..3], &data[0..3]);
            unsafe {
                assert!(slice_consumer.did_consume(3).await.is_ok());
            }

            // Make a second call to `did_consume` which exceeds the number of available slots.
            unsafe {
                let _ = slice_consumer.did_consume(2).await;
            }
        })
    }

    #[test]
    fn errors_on_consumer_slots_when_none_are_available() {
        smol::block_on(async {
            // Create a slice consumer that exposes four slots.
            let mut buf = [0; 4];
            let mut slice_consumer = SliceConsumer::new(&mut buf);

            // Copy data to two of the available slots and call `did_consume`.
            let data = b"tofu";
            let slots = slice_consumer.consumer_slots().await.unwrap();
            MaybeUninit::copy_from_slice(&mut slots[0..2], &data[0..2]);
            unsafe {
                assert!(slice_consumer.did_consume(2).await.is_ok());
            }

            // Copy data to two of the available slots and call `did_consume`.
            let slots = slice_consumer.consumer_slots().await.unwrap();
            MaybeUninit::copy_from_slice(&mut slots[0..2], &data[0..2]);
            unsafe {
                assert!(slice_consumer.did_consume(2).await.is_ok());
            }

            // Make a third call to `consumer_slots` after all available slots have been used.
            assert_eq!(
                slice_consumer.consumer_slots().await.unwrap_err(),
                SliceConsumerFullError
            );
        })
    }

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
        smol::block_on(async {
            let mut into_vec = IntoVec::new();
            let _ = into_vec.close(()).await;
            let _ = into_vec.consume(7).await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_close_after_close() {
        smol::block_on(async {
            // Type annotations are required because we never provide a `T`.
            let mut into_vec: IntoVec<u8> = IntoVec::new();
            let _ = into_vec.close(()).await;
            let _ = into_vec.close(()).await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_flush_after_close() {
        smol::block_on(async {
            let mut into_vec: IntoVec<u8> = IntoVec::new();
            let _ = into_vec.close(()).await;
            let _ = into_vec.flush().await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_consumer_slots_after_close() {
        smol::block_on(async {
            let mut into_vec: IntoVec<u8> = IntoVec::new();
            let _ = into_vec.close(()).await;
            let _ = into_vec.consumer_slots().await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_did_consume_after_close() {
        smol::block_on(async {
            let mut into_vec: IntoVec<u8> = IntoVec::new();
            let _ = into_vec.close(()).await;

            unsafe {
                let _ = into_vec.did_consume(7).await;
            }
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_bulk_consume_after_close() {
        smol::block_on(async {
            let mut into_vec = IntoVec::new();
            let _ = into_vec.close(()).await;
            let _ = into_vec.bulk_consume(b"ufo").await;
        })
    }

    #[test]
    #[should_panic(
        expected = "may not call `did_consume` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_consume_with_amount_greater_than_available_slots() {
        smol::block_on(async {
            let mut into_vec: IntoVec<u8> = IntoVec::new();

            unsafe {
                let _ = into_vec.did_consume(21).await;
            }
        })
    }
}
