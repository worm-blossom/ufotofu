use core::convert::{AsMut, AsRef};
use core::mem::MaybeUninit;

use thiserror::Error;
use wrapper::Wrapper;

use crate::local_nb::consumer::SyncToLocalNb;
use crate::local_nb::{BufferedConsumer, BulkConsumer, Consumer};
use crate::sync::consumer::SliceConsumer as SyncSliceConsumer;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[error("slice consumer is full")]
/// Error to indicate that consuming data into a slice failed because the end of the slice was reached.
pub struct SliceConsumerFullError;

#[derive(Debug)]
pub struct SliceConsumer<'a, T>(SyncToLocalNb<SyncSliceConsumer<'a, T>>);

/// Creates a consumer which places consumed data into the given slice.
impl<'a, T> SliceConsumer<'a, T> {
    pub fn new(slice: &mut [T]) -> SliceConsumer<'_, T> {
        let slice_consumer = SyncSliceConsumer::new(slice);

        SliceConsumer(SyncToLocalNb(slice_consumer))
    }
}

impl<'a, T> AsRef<[T]> for SliceConsumer<'a, T> {
    fn as_ref(&self) -> &[T] {
        let inner = self.0.as_ref();
        inner.as_ref()
    }
}

impl<'a, T> AsMut<[T]> for SliceConsumer<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        let inner = self.0.as_mut();
        inner.as_mut()
    }
}

impl<'a, T> Wrapper<&'a [T]> for SliceConsumer<'a, T> {
    fn into_inner(self) -> &'a [T] {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<'a, T> Consumer for SliceConsumer<'a, T> {
    /// The type of the items to be consumed.
    type Item = T;
    /// The value signifying the end of the consumed sequence.
    type Final = ();
    /// The value emitted when the consumer is full and a subsequent
    /// call is made to `consume()` or `consumer_slots()`.
    type Error = SliceConsumerFullError;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.0
            .consume(item)
            .await
            .map_err(|_| SliceConsumerFullError)
    }

    async fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        self.0
            .close(final_val)
            .await
            .map_err(|_| SliceConsumerFullError)
    }
}

impl<'a, T> BufferedConsumer for SliceConsumer<'a, T> {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush().await.map_err(|_| SliceConsumerFullError)
    }
}

impl<'a, T: Copy> BulkConsumer for SliceConsumer<'a, T> {
    async fn consumer_slots<'b>(
        &'b mut self,
    ) -> Result<&'b mut [MaybeUninit<Self::Item>], Self::Error>
    where
        T: 'b,
    {
        self.0
            .consumer_slots()
            .await
            .map_err(|_| SliceConsumerFullError)
    }

    async unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0
            .did_consume(amount)
            .await
            .map_err(|_| SliceConsumerFullError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            let mut buf = [0; 1];

            let mut slice_consumer = SliceConsumer::new(&mut buf);
            let _ = slice_consumer.close(()).await;
            let _ = slice_consumer.consume(7).await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_close_after_close() {
        smol::block_on(async {
            let mut buf = [0; 1];

            let mut slice_consumer = SliceConsumer::new(&mut buf);
            let _ = slice_consumer.close(()).await;
            let _ = slice_consumer.close(()).await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_flush_after_close() {
        smol::block_on(async {
            let mut buf = [0; 1];

            let mut slice_consumer = SliceConsumer::new(&mut buf);
            let _ = slice_consumer.close(()).await;
            let _ = slice_consumer.flush().await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_consumer_slots_after_close() {
        smol::block_on(async {
            let mut buf = [0; 1];

            let mut slice_consumer = SliceConsumer::new(&mut buf);
            let _ = slice_consumer.close(()).await;
            let _ = slice_consumer.consumer_slots().await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_did_consume_after_close() {
        smol::block_on(async {
            let mut buf = [0; 8];

            let mut slice_consumer = SliceConsumer::new(&mut buf);
            let _ = slice_consumer.close(()).await;

            unsafe {
                let _ = slice_consumer.did_consume(7).await;
            }
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_bulk_consume_after_close() {
        smol::block_on(async {
            let mut buf = [0; 8];

            let mut slice_consumer = SliceConsumer::new(&mut buf);
            let _ = slice_consumer.close(()).await;
            let _ = slice_consumer.bulk_consume(b"ufo").await;
        })
    }

    #[test]
    #[should_panic(
        expected = "may not call `did_consume` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_consume_with_amount_greater_than_available_slots() {
        smol::block_on(async {
            let mut buf = [0; 8];

            let mut slice_consumer = SliceConsumer::new(&mut buf);

            unsafe {
                let _ = slice_consumer.did_consume(21).await;
            }
        })
    }
}
