use core::convert::Infallible;
use core::fmt::Debug;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{
    alloc::{Allocator, Global},
    vec::Vec,
};
#[cfg(feature = "std")]
use std::vec::Vec;

use crate::consumer::IntoVecFallible;

use crate::{BufferedConsumer, BulkConsumer, Consumer};

#[derive(Debug, Clone)]
/// Collects data and can at any point be converted into a `Vec<T>`.
pub struct IntoVec<T>(IntoVecFallible<T>);

impl<T: Default> Default for IntoVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntoVec<T> {
    /// Creates a new consumer that collects data into a Vec.
    pub fn new() -> IntoVec<T> {
        IntoVec(IntoVecFallible::new())
    }

    /// Converts `self` into the vector of all consumed items.
    pub fn into_vec(self) -> Vec<T> {
        self.0.into_vec()
    }
}

impl<T> AsRef<[T]> for IntoVec<T> {
    fn as_ref(&self) -> &[T] {
        self.0.as_ref()
    }
}

impl<T: Default> IntoVec<T> {
    pub(crate) fn remaining_slots(&self) -> usize {
        self.0.remaining_slots()
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

    async fn consume(&mut self, item: T) -> Result<Self::Final, Self::Error> {
        Consumer::consume(&mut self.0, item)
            .await
            .expect("Out of memory");
        Ok(())
    }

    async fn close(&mut self, fin: Self::Final) -> Result<Self::Final, Self::Error> {
        Consumer::close(&mut self.0, fin)
            .await
            .expect("Out of memory");
        Ok(())
    }
}

impl<T: Default> BufferedConsumer for IntoVec<T> {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        BufferedConsumer::flush(&mut self.0)
            .await
            .expect("Out of memory");
        Ok(())
    }
}

impl<T: Default + Copy> BulkConsumer for IntoVec<T> {
    async fn expose_slots<'a>(&'a mut self) -> Result<&'a mut [Self::Item], Self::Error>
    where
        Self::Item: 'a,
    {
        Ok(BulkConsumer::expose_slots(&mut self.0)
            .await
            .expect("Out of memory"))
    }

    async fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        BulkConsumer::consume_slots(&mut self.0, amount)
            .await
            .expect("Out of memory");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::*;

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
}
