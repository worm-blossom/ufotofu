use core::fmt::Debug;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{
    alloc::{Allocator, Global},
    collections::TryReserveError,
    vec::Vec,
};
#[cfg(feature = "std")]
use std::{collections::TryReserveError, vec::Vec};

use wrapper::Wrapper;

use crate::consumer::Invariant;
use crate::{BufferedConsumer, BulkConsumer, Consumer};

#[derive(Clone)]
/// Collects data and can at any point be converted into a `Vec<T>`. Unlike [`IntoVec`](crate::sync::consumer::IntoVec), reports an error instead of panicking when an internal memory allocation fails.
pub struct IntoVecFallible_<T>(Invariant<IntoVecFallible<T>>);

invarianted_impl_debug!(IntoVecFallible_<T: Debug>);

impl<T> Default for IntoVecFallible_<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntoVecFallible_<T> {
    pub fn new() -> IntoVecFallible_<T> {
        let invariant = Invariant::new(IntoVecFallible {
            v: Vec::new(),
            consumed: 0,
        });

        IntoVecFallible_(invariant)
    }

    pub fn into_vec(self) -> Vec<T> {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<T: Default> IntoVecFallible_<T> {
    pub(crate) fn make_space_even_if_not_needed(&mut self) -> Result<(), TryReserveError> {
        self.0.as_mut().make_space_even_if_not_needed()
    }

    pub(crate) fn remaining_slots(&self) -> usize {
        self.0.as_ref().remaining_slots()
    }
}

invarianted_impl_as_ref!(IntoVecFallible_<T>; [T]);
invarianted_impl_wrapper!(IntoVecFallible_<T>; Vec<T>);

invarianted_impl_consumer!(IntoVecFallible_<T: Default> Item T; Final (); Error TryReserveError);
invarianted_impl_buffered_consumer!(IntoVecFallible_<T: Default>);
invarianted_impl_bulk_consumer!(IntoVecFallible_<T: Copy + Default>);

#[derive(Debug, Clone)]
struct IntoVecFallible<T> {
    v: Vec<T>,
    consumed: usize,
}

impl<T> AsRef<[T]> for IntoVecFallible<T> {
    fn as_ref(&self) -> &[T] {
        &self.v[..self.consumed]
    }
}

impl<T> Wrapper<Vec<T>> for IntoVecFallible<T> {
    fn into_inner(self) -> Vec<T> {
        let IntoVecFallible { mut v, consumed } = self;
        v.truncate(consumed);
        v
    }
}

impl<T: Default> IntoVecFallible<T> {
    fn make_space_if_needed(&mut self) -> Result<(), TryReserveError> {
        // Allocate additional capacity to the vector if no empty slots are available.
        if self.consumed == self.v.len() {
            // Will return an error if capacity overflows or the allocator reports a failure.
            self.v.try_reserve(self.consumed + 1)?;
            self.v.resize_with(self.consumed * 2 + 1, Default::default); // Does not allocate because we reserved before.
        }

        Ok(())
    }

    fn make_space_even_if_not_needed(&mut self) -> Result<(), TryReserveError> {
        // Will return an error if capacity overflows or the allocator reports a failure.
        self.v.try_reserve(self.v.len() + 1)?;
        self.v.resize_with(self.v.len() * 2 + 1, Default::default); // Does not allocate because we reserved before.

        Ok(())
    }

    fn remaining_slots(&self) -> usize {
        self.v.len() - self.consumed
    }
}

impl<T: Default> Consumer for IntoVecFallible<T> {
    type Item = T;
    type Final = ();
    type Error = TryReserveError;

    async fn consume(&mut self, item: T) -> Result<Self::Final, Self::Error> {
        // Allocate additional capacity to the vector if no empty slots are available.
        self.make_space_if_needed()?;

        self.v[self.consumed] = item;
        self.consumed += 1;

        Ok(())
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<Self::Final, Self::Error> {
        Ok(())
    }
}

impl<T: Default> BufferedConsumer for IntoVecFallible<T> {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T: Default + Copy> BulkConsumer for IntoVecFallible<T> {
    async fn expose_slots<'a>(&'a mut self) -> Result<&'a mut [Self::Item], Self::Error>
    where
        Self::Item: 'a,
    {
        // Allocate additional capacity to the vector if no empty slots are available.
        self.make_space_if_needed()?;

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
        let consumer: IntoVecFallible<u8> = IntoVecFallible::new();
        assert_eq!(
            std::format!("{:?}", consumer),
            "IntoVecFallible { v: [], consumed: 0 }"
        );
    }

    #[test]
    fn converts_into_vec() {
        let mut into_vec = IntoVecFallible::new();
        let _ = into_vec.bulk_consume_full_slice(b"ufotofu");
        let _ = into_vec.close(());

        let v = into_vec.into_vec();
        assert_eq!(v, std::vec![117, 102, 111, 116, 111, 102, 117]);
    }
}
