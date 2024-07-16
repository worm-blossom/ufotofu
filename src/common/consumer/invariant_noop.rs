use core::mem::MaybeUninit;

use wrapper::Wrapper;

use crate::local_nb::{
    BufferedConsumer as BufferedConsumerLocalNb, BulkConsumer as BulkConsumerLocalNb,
    Consumer as ConsumerLocalNb,
};
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
///   - slots
///   - `consume_slots`
///   - `bulk_consume`
/// - Must not call any of the prior functions after any of them had returned
///   an error.
/// - Must not call `consume_slots` for slots that had not been exposed by
///   slots before.
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
    /// Return a `Consumer` that behaves exactly like the wrapped `Consumer`
    /// `inner`, except that - when running tests - it performs runtime
    /// validation of API invariants and panics if they are violated by a
    /// caller.
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
    fn expose_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        self.inner.expose_slots()
    }

    unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.inner.consume_slots(amount)
    }
}

impl<C: ConsumerLocalNb> ConsumerLocalNb for Invariant<C> {
    type Item = C::Item;
    type Final = C::Final;
    type Error = C::Error;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.inner.consume(item).await
    }

    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error> {
        self.inner.close(fin).await
    }
}

impl<C: BufferedConsumerLocalNb> BufferedConsumerLocalNb for Invariant<C> {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush().await
    }
}

impl<C: BulkConsumerLocalNb> BulkConsumerLocalNb for Invariant<C>
where
    C::Item: Copy,
{
    async fn expose_slots<'a>(
        &'a mut self,
    ) -> Result<&'a mut [MaybeUninit<Self::Item>], Self::Error>
    where
        Self::Item: 'a,
    {
        self.inner.expose_slots().await
    }

    async unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.inner.consume_slots(amount).await
    }
}

#[cfg(test)]
mod tests {
    // use super::super::*;

    // use crate::sync::consumer::{IntoVec, SliceConsumer, SliceConsumerFullError};
    // use crate::sync::*;

    // #[test]
    // fn accepts_valid_did_consume_amount() {
    //     // Create a slice consumer that exposes four slots.
    //     let mut buf = [0; 4];
    //     let mut slice_consumer = SliceConsumer::new(&mut buf);

    //     // Copy data to three of the available slots and call `consume_slots`.
    //     let data = b"ufo";
    //     let slots = slice_consumer.expose_slots().unwrap();
    //     MaybeUninit::copy_from_slice(&mut slots[0..3], &data[0..3]);
    //     unsafe {
    //         assert!(slice_consumer.consume_slots(3).is_ok());
    //     }
    // }

    // #[test]
    // #[should_panic(
    //     expected = "may not call `consume_slots` with an amount exceeding the total number of exposed slots"
    // )]
    // fn panics_on_second_did_consume_with_amount_greater_than_available_slots() {
    //     // Create a slice consumer that exposes four slots.
    //     let mut buf = [0; 4];
    //     let mut slice_consumer = SliceConsumer::new(&mut buf);

    //     // Copy data to three of the available slots and call `consume_slots`.
    //     let data = b"ufo";
    //     let slots = slice_consumer.expose_slots().unwrap();
    //     MaybeUninit::copy_from_slice(&mut slots[0..3], &data[0..3]);
    //     unsafe {
    //         assert!(slice_consumer.consume_slots(3).is_ok());
    //     }

    //     // Make a second call to `consume_slots` which exceeds the number of available slots.
    //     unsafe {
    //         let _ = slice_consumer.consume_slots(2);
    //     }
    // }

    // #[test]
    // fn errors_on_consumer_slots_when_none_are_available() {
    //     // Create a slice consumer that exposes four slots.
    //     let mut buf = [0; 4];
    //     let mut slice_consumer = SliceConsumer::new(&mut buf);

    //     // Copy data to two of the available slots and call `consume_slots`.
    //     let data = b"tofu";
    //     let slots = slice_consumer.expose_slots().unwrap();
    //     MaybeUninit::copy_from_slice(&mut slots[0..2], &data[0..2]);
    //     unsafe {
    //         assert!(slice_consumer.consume_slots(2).is_ok());
    //     }

    //     // Copy data to two of the available slots and call `consume_slots`.
    //     let slots = slice_consumer.expose_slots().unwrap();
    //     MaybeUninit::copy_from_slice(&mut slots[0..2], &data[0..2]);
    //     unsafe {
    //         assert!(slice_consumer.consume_slots(2).is_ok());
    //     }

    //     // Make a third call to slots after all available slots have been used.
    //     assert_eq!(
    //         slice_consumer.expose_slots().unwrap_err(),
    //         SliceConsumerFullError
    //     );
    // }

    // // Panic conditions:
    // //
    // // - `consume()` must not be called after `close()` or error
    // // - `close()` must not be called after `close()` or error
    // // - `flush()` must not be called after `close()` or error
    // // - `consumer_slots()` must not be called after `close()` or error
    // // - `did_consume()` must not be called after `close()` or error
    // // - `bulk_consume()` must not be called after `close()` or error
    // // - `did_consume(amount)` must not be called with `amount` greater than available slots

    // // In each of the following tests, the final function call should panic.

    // #[test]
    // #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    // fn panics_on_consume_after_close() {
    //     let mut into_vec = IntoVec::new();
    //     let _ = into_vec.close(());
    //     let _ = into_vec.consume(7);
    // }

    // #[test]
    // #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    // fn panics_on_close_after_close() {
    //     // Type annotations are required because we never provide a `T`.
    //     let mut into_vec: IntoVec<u8> = IntoVec::new();
    //     let _ = into_vec.close(());
    //     let _ = into_vec.close(());
    // }

    // #[test]
    // #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    // fn panics_on_flush_after_close() {
    //     let mut into_vec: IntoVec<u8> = IntoVec::new();
    //     let _ = into_vec.close(());
    //     let _ = into_vec.flush();
    // }

    // #[test]
    // #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    // fn panics_on_consumer_slots_after_close() {
    //     let mut into_vec: IntoVec<u8> = IntoVec::new();
    //     let _ = into_vec.close(());
    //     let _ = into_vec.expose_slots();
    // }

    // #[test]
    // #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    // fn panics_on_did_consume_after_close() {
    //     let mut into_vec: IntoVec<u8> = IntoVec::new();
    //     let _ = into_vec.close(());

    //     unsafe {
    //         let _ = into_vec.consume_slots(7);
    //     }
    // }

    // #[test]
    // #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    // fn panics_on_bulk_consume_after_close() {
    //     let mut into_vec = IntoVec::new();
    //     let _ = into_vec.close(());
    //     let _ = into_vec.bulk_consume(b"ufo");
    // }

    // #[test]
    // #[should_panic(
    //     expected = "may not call `consume_slots` with an amount exceeding the total number of exposed slots"
    // )]
    // fn panics_on_did_consume_with_amount_greater_than_available_slots() {
    //     let mut into_vec: IntoVec<u8> = IntoVec::new();

    //     unsafe {
    //         let _ = into_vec.consume_slots(21);
    //     }
    // }
}
