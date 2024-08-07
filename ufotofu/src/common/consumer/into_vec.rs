use core::fmt::Debug;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{
    alloc::{Allocator, Global},
    vec::Vec,
};
#[cfg(feature = "std")]
use std::{
    alloc::{Allocator, Global},
    vec::Vec,
};

use wrapper::Wrapper;

use crate::common::consumer::IntoVecFallible;

use crate::local_nb::{
    BufferedConsumer as BufferedConsumerLocalNb, BulkConsumer as BulkConsumerLocalNb,
    Consumer as ConsumerLocalNb,
};
use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

/// Collects data and can at any point be converted into a `Vec<T>`.
pub struct IntoVec<T, A: Allocator = Global>(IntoVecFallible<T, A>);

impl<T: Debug, A: Allocator + Debug> core::fmt::Debug for IntoVec<T, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Default> Default for IntoVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntoVec<T> {
    /// Create a new consumer that collects data into a Vec.
    pub fn new() -> IntoVec<T> {
        IntoVec(IntoVecFallible::new())
    }

    /// Convert `self` into the vector of all consumed items.
    pub fn into_vec(self) -> Vec<T> {
        self.0.into_inner()
    }
}

impl<T, A: Allocator> IntoVec<T, A> {
    pub fn new_in(alloc: A) -> IntoVec<T, A> {
        IntoVec(IntoVecFallible::new_in(alloc))
    }
}

impl<T, A: Allocator> AsRef<Vec<T, A>> for IntoVec<T, A> {
    fn as_ref(&self) -> &Vec<T, A> {
        self.0.as_ref()
    }
}

impl<T, A: Allocator> AsMut<Vec<T, A>> for IntoVec<T, A> {
    fn as_mut(&mut self) -> &mut Vec<T, A> {
        self.0.as_mut()
    }
}

impl<T, A: Allocator> Wrapper<Vec<T, A>> for IntoVec<T, A> {
    fn into_inner(self) -> Vec<T, A> {
        self.0.into_inner()
    }
}

impl<T: Default, A: Allocator> Consumer for IntoVec<T, A> {
    type Item = T;
    type Final = ();
    type Error = !;

    fn consume(&mut self, item: T) -> Result<Self::Final, Self::Error> {
        Ok(Consumer::consume(&mut self.0, item).expect("Out of memory"))
    }

    fn close(&mut self, fin: Self::Final) -> Result<Self::Final, Self::Error> {
        Ok(Consumer::close(&mut self.0, fin).expect("Out of memory"))
    }
}

impl<T: Default, A: Allocator> BufferedConsumer for IntoVec<T, A> {
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(BufferedConsumer::flush(&mut self.0).expect("Out of memory"))
    }
}

impl<T: Default + Copy, A: Allocator> BulkConsumer for IntoVec<T, A> {
    fn expose_slots(&mut self) -> Result<&mut [Self::Item], Self::Error> {
        Ok(BulkConsumer::expose_slots(&mut self.0).expect("Out of memory"))
    }

    fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        Ok(BulkConsumer::consume_slots(&mut self.0, amount).expect("Out of memory"))
    }
}

impl<T: Default, A: Allocator> ConsumerLocalNb for IntoVec<T, A> {
    type Item = T;
    type Final = ();
    type Error = !;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        Ok(ConsumerLocalNb::consume(&mut self.0, item)
            .await
            .expect("Out of memory"))
    }

    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error> {
        Ok(ConsumerLocalNb::close(&mut self.0, fin)
            .await
            .expect("Out of memory"))
    }
}

impl<T: Default, A: Allocator> BufferedConsumerLocalNb for IntoVec<T, A> {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(BufferedConsumerLocalNb::flush(&mut self.0)
            .await
            .expect("Out of memory"))
    }
}

impl<T: Default, A: Allocator> BulkConsumerLocalNb for IntoVec<T, A>
where
    T: Copy,
{
    async fn expose_slots<'a>(&'a mut self) -> Result<&'a mut [Self::Item], Self::Error>
    where
        Self::Item: 'a,
    {
        Ok(BulkConsumerLocalNb::expose_slots(&mut self.0)
            .await
            .expect("Out of memory"))
    }

    async fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        Ok(BulkConsumerLocalNb::consume_slots(&mut self.0, amount)
            .await
            .expect("Out of memory"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::sync::*;

    use std::format;

    // The debug output hides the internals of using semantically transparent wrappers.
    #[test]
    fn debug_output_hides_transparent_wrappers() {
        let consumer: IntoVec<u8> = IntoVec::new();
        assert_eq!(format!("{:?}", consumer), "IntoVec([])");
    }

    #[test]
    fn converts_into_vec() {
        let mut into_vec = IntoVec::new();
        let _ = into_vec.bulk_consume(b"ufotofu");
        let _ = into_vec.close(());

        let vec = into_vec.into_vec();
        assert_eq!(vec.len(), 7);
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
        let mut into_vec = IntoVec::new();
        let _ = into_vec.close(());
        let _ = into_vec.consume(7);
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_close_after_close() {
        // Type annotations are required because we never provide a `T`.
        let mut into_vec: IntoVec<u8> = IntoVec::new();
        let _ = into_vec.close(());
        let _ = into_vec.close(());
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_flush_after_close() {
        let mut into_vec: IntoVec<u8> = IntoVec::new();
        let _ = into_vec.close(());
        let _ = into_vec.flush();
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_consumer_slots_after_close() {
        let mut into_vec: IntoVec<u8> = IntoVec::new();
        let _ = into_vec.close(());
        let _ = into_vec.expose_slots();
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_did_consume_after_close() {
        let mut into_vec: IntoVec<u8> = IntoVec::new();
        let _ = into_vec.close(());

        let _ = into_vec.consume_slots(7);
    }

    #[test]
    #[should_panic(expected = "may not call `Consumer` methods after the sequence has ended")]
    fn panics_on_bulk_consume_after_close() {
        let mut into_vec = IntoVec::new();
        let _ = into_vec.close(());
        let _ = into_vec.bulk_consume(b"ufo");
    }

    #[test]
    #[should_panic(
        expected = "may not call `consume_slots` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_consume_with_amount_greater_than_available_slots() {
        let mut into_vec: IntoVec<u8> = IntoVec::new();

        let _ = into_vec.consume_slots(21);
    }
}
