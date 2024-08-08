use core::convert::Infallible;
use core::fmt::Debug;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{
    alloc::{Allocator, Global},
    vec::Vec,
};
#[cfg(feature = "std")]
use std::vec::Vec;

use wrapper::Wrapper;

use crate::common::consumer::IntoVecFallible;

use crate::local_nb::{
    BufferedConsumer as BufferedConsumerLocalNb, BulkConsumer as BulkConsumerLocalNb,
    Consumer as ConsumerLocalNb,
};
use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

#[derive(Debug)]
/// Collects data and can at any point be converted into a `Vec<T>`.
pub struct IntoVec<T>(IntoVecFallible<T>);

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

impl<T> AsRef<[T]> for IntoVec<T> {
    fn as_ref(&self) -> &[T] {
        self.0.as_ref()
    }
}

impl<T> Wrapper<Vec<T>> for IntoVec<T> {
    fn into_inner(self) -> Vec<T> {
        self.0.into_inner()
    }
}

impl<T: Default> IntoVec<T> {
    pub(crate) fn remaining_slots(&self) -> usize {
        self.0.remaining_slots()
    }

    pub(crate) fn make_space_if_needed(&mut self) {
        self.0.make_space_if_needed().expect("Out of memory")
    }

    pub(crate) fn make_space_even_if_not_needed(&mut self) {
        self.0
            .make_space_even_if_not_needed()
            .expect("Out of memory")
    }
}

impl<T: Default> Consumer for IntoVec<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;

    fn consume(&mut self, item: T) -> Result<Self::Final, Self::Error> {
        Consumer::consume(&mut self.0, item).expect("Out of memory");
        Ok(())
    }

    fn close(&mut self, fin: Self::Final) -> Result<Self::Final, Self::Error> {
        Consumer::close(&mut self.0, fin).expect("Out of memory");
        Ok(())
    }
}

impl<T: Default> BufferedConsumer for IntoVec<T> {
    fn flush(&mut self) -> Result<(), Self::Error> {
        BufferedConsumer::flush(&mut self.0).expect("Out of memory");
        Ok(())
    }
}

impl<T: Default + Copy> BulkConsumer for IntoVec<T> {
    fn expose_slots(&mut self) -> Result<&mut [Self::Item], Self::Error> {
        Ok(BulkConsumer::expose_slots(&mut self.0).expect("Out of memory"))
    }

    fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        BulkConsumer::consume_slots(&mut self.0, amount).expect("Out of memory");
        Ok(())
    }
}

impl<T: Default> ConsumerLocalNb for IntoVec<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        ConsumerLocalNb::consume(&mut self.0, item)
            .await
            .expect("Out of memory");
        Ok(())
    }

    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error> {
        ConsumerLocalNb::close(&mut self.0, fin)
            .await
            .expect("Out of memory");
        Ok(())
    }
}

impl<T: Default> BufferedConsumerLocalNb for IntoVec<T> {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        BufferedConsumerLocalNb::flush(&mut self.0)
            .await
            .expect("Out of memory");
        Ok(())
    }
}

impl<T: Default> BulkConsumerLocalNb for IntoVec<T>
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
        BulkConsumerLocalNb::consume_slots(&mut self.0, amount)
            .await
            .expect("Out of memory");
        Ok(())
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
        assert_eq!(
            format!("{:?}", consumer),
            "IntoVec(IntoVecFallible { v: [], consumed: 0 })"
        );
    }

    #[test]
    fn converts_into_vec() {
        let mut into_vec = IntoVec::new();
        let _ = into_vec.bulk_consume_full_slice(b"ufotofu");
        let _ = into_vec.close(());

        let v = into_vec.into_vec();
        assert_eq!(v, std::vec![117, 102, 111, 116, 111, 102, 117]);
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
