use core::{convert::Infallible, fmt::Debug};

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{
    alloc::{Allocator, Global},
    vec::Vec,
};
#[cfg(feature = "std")]
use std::vec::Vec;

use crate::consumer::Invariant;
use crate::{BufferedConsumer, BulkConsumer, Consumer};

#[derive(Clone)]
/// Collects data and can at any point be converted into a `Vec<T>`.
pub struct IntoVec_<T>(Invariant<IntoVec<T>>);

invarianted_impl_debug!(IntoVec_<T: Debug>);

impl<T> Default for IntoVec_<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntoVec_<T> {
    /// Creates a new consumer that collects data into a Vec.
    ///
    /// ```
    /// use ufotofu::consumer::*;
    /// use ufotofu::*;
    ///
    /// let mut intoVec = IntoVec::new();
    ///
    /// pollster::block_on(async {
    ///     assert_eq!(Ok(()), intoVec.consume(4).await);
    ///     assert_eq!(Ok(()), intoVec.consume(7).await);
    ///     assert_eq!(vec![4, 7], intoVec.into_vec());
    /// });
    /// ```
    pub fn new() -> IntoVec_<T> {
        Self::with_capacity(0)
    }

    /// Creates a new consumer that collects data into a Vec, which starts with at least the given starting capacity.
    ///
    /// ```
    /// use ufotofu::consumer::*;
    /// use ufotofu::*;
    ///
    /// let mut intoVec = IntoVec::with_capacity(999);
    ///
    /// pollster::block_on(async {
    ///     assert_eq!(Ok(()), intoVec.consume(4).await);
    ///     assert_eq!(Ok(()), intoVec.consume(7).await);
    ///
    ///     let collected = intoVec.into_vec();
    ///     assert!(collected.capacity() >= 999);
    ///     assert_eq!(vec![4, 7], collected);
    /// });
    /// ```
    pub fn with_capacity(capacity: usize) -> IntoVec_<T> {
        let invariant = Invariant::new(IntoVec {
            v: Vec::with_capacity(capacity),
            consumed: 0,
        });

        IntoVec_(invariant)
    }

    /// Converts `self` into the vector of all consumed items.
    ///
    /// ```
    /// use ufotofu::consumer::*;
    /// use ufotofu::*;
    ///
    /// let mut intoVec = IntoVec::new();
    ///
    /// pollster::block_on(async {
    ///     assert_eq!(Ok(()), intoVec.consume(4).await);
    ///     assert_eq!(Ok(()), intoVec.consume(7).await);
    ///     assert_eq!(vec![4, 7], intoVec.into_vec());
    /// });
    /// ```
    pub fn into_vec(self) -> Vec<T> {
        let inner = self.0.into_inner();
        inner.into_vec()
    }
}

impl<T: Default> IntoVec_<T> {
    pub(crate) fn make_space_even_if_not_needed(&mut self) {
        self.0.as_mut().make_space_even_if_not_needed()
    }

    pub(crate) fn remaining_slots(&self) -> usize {
        self.0.as_ref().remaining_slots()
    }
}

invarianted_impl_as_ref!(IntoVec_<T>; [T]);

invarianted_impl_consumer!(IntoVec_<T: Default> Item T; Final (); Error Infallible);
invarianted_impl_buffered_consumer!(IntoVec_<T: Default>);
invarianted_impl_bulk_consumer!(IntoVec_<T: Default>);

#[derive(Debug, Clone)]
struct IntoVec<T> {
    v: Vec<T>,
    consumed: usize,
}

impl<T> AsRef<[T]> for IntoVec<T> {
    fn as_ref(&self) -> &[T] {
        &self.v[..self.consumed]
    }
}

impl<T> IntoVec<T> {
    /// Converts `self` into the vector of all consumed items.
    fn into_vec(self) -> Vec<T> {
        let IntoVec { mut v, consumed } = self;
        v.truncate(consumed);
        v
    }
}

impl<T: Default> IntoVec<T> {
    fn make_space_if_needed(&mut self) {
        // Allocate additional capacity to the vector if no empty slots are available.
        if self.consumed == self.v.len() {
            self.v.resize_with(self.consumed * 2 + 1, Default::default);
        }
    }

    fn make_space_even_if_not_needed(&mut self) {
        self.v.resize_with(self.v.len() * 2 + 1, Default::default);
    }

    fn remaining_slots(&self) -> usize {
        self.v.len() - self.consumed
    }
}

impl<T: Default> Consumer for IntoVec<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;

    async fn consume(&mut self, item: T) -> Result<Self::Final, Self::Error> {
        // Allocate additional capacity to the vector if no empty slots are available.
        self.make_space_if_needed();

        self.v[self.consumed] = item;
        self.consumed += 1;

        Ok(())
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<Self::Final, Self::Error> {
        Ok(())
    }
}

impl<T: Default> BufferedConsumer for IntoVec<T> {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T: Default> BulkConsumer for IntoVec<T> {
    async fn expose_slots<'a>(&'a mut self) -> Result<&'a mut [Self::Item], Self::Error>
    where
        Self::Item: 'a,
    {
        // Allocate additional capacity to the vector if no empty slots are available.
        self.make_space_if_needed();

        Ok(&mut self.v[self.consumed..])
    }

    async fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.consumed += amount;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::*;

    // The debug output hides the internals of using semantically transparent wrappers.
    #[test]
    fn debug_output_hides_transparent_wrappers() {
        let consumer: IntoVec<u8> = IntoVec::new();
        assert_eq!(
            std::format!("{:?}", consumer),
            "IntoVec { v: [], consumed: 0 }"
        );
    }

    #[test]
    fn converts_into_vec() {
        pollster::block_on(async {
            let mut into_vec = IntoVec::new();
            assert_eq!(Ok(()), into_vec.bulk_consume_full_slice(b"ufotofu").await);
            assert_eq!(Ok(()), into_vec.close(()).await);

            let v = into_vec.into_vec();
            assert_eq!(v, std::vec![117, 102, 111, 116, 111, 102, 117]);
        });
    }
}
