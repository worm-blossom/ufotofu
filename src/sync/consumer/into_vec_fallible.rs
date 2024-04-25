use core::mem::MaybeUninit;
use core::slice;

use std::alloc::{Allocator, Global};
use std::collections;
use std::vec::Vec;

use thiserror::Error;
use wrapper::Wrapper;

use crate::maybe_uninit_slice_mut;
use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

#[derive(Debug, Error)]
pub enum IntoVecError {
    #[error("An infallible action failed")]
    Never,
    #[error(transparent)]
    TryReserve(#[from] collections::TryReserveError),
}

impl From<!> for IntoVecError {
    fn from(_never: !) -> IntoVecError {
        Self::Never
    }
}

/// A fallible implementation of `IntoVec` which returns an error
/// if there is insufficient memory to (re)allocate the inner
/// vector or if the allocator reports a failure.
///
/// Collects data and can at any point be converted into a `Vec<T>.
pub struct IntoVecFallible<T, A = Global>
where
    A: Allocator,
{
    v: Vec<T, A>,
}

impl<T> IntoVecFallible<T> {
    pub fn new() -> IntoVecFallible<T> {
        IntoVecFallible { v: Vec::new() }
    }

    pub fn into_vec(self) -> Vec<T> {
        self.v
    }
}

impl<T, A> IntoVecFallible<T, A>
where
    A: Allocator,
{
    pub fn new_in(alloc: A) -> IntoVecFallible<T, A> {
        IntoVecFallible {
            v: Vec::new_in(alloc),
        }
    }
}

impl<T> Consumer for IntoVecFallible<T> {
    type Item = T;
    type Final = ();
    type Error = IntoVecError;

    fn consume(&mut self, item: T) -> Result<Self::Final, Self::Error> {
        if let Err(value) = self.v.push_within_capacity(item) {
            self.v.try_reserve(1)?;
            // This cannot fail; the previous line either returned or added
            // at least 1 free slot.
            let _ = self.v.push_within_capacity(value);
        }

        Ok(())
    }

    fn close(&mut self, _final: Self::Final) -> Result<Self::Final, Self::Error> {
        Ok(())
    }
}

impl<T: Copy> BufferedConsumer for IntoVecFallible<T> {
    fn flush(&mut self) -> Result<Self::Final, Self::Error> {
        Ok(())
    }
}

impl<T: Copy> BulkConsumer for IntoVecFallible<T> {
    fn consumer_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        // Allocate additional capacity to the vector if no empty slots are available.
        if self.v.capacity() == self.v.len() {
            // Will return an error if capacity overflows or the allocator reports a failure.
            self.v.try_reserve((self.v.capacity() * 2) + 1)?;
        }

        let pointer = self.v.as_mut_ptr();
        let available_capacity = self.v.capacity() - self.v.len();

        unsafe {
            // Return a mutable slice which represents available (unused) slots.
            Ok(maybe_uninit_slice_mut(slice::from_raw_parts_mut(
                // The pointer offset.
                pointer.add(self.v.len()),
                // The length (number of slots).
                available_capacity,
            )))
        }
    }

    unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        // Update the length of the vector based on the amount of items consumed.
        self.v.set_len(self.v.len() + amount);

        Ok(())
    }
}

impl<T> Wrapper<Vec<T>> for IntoVecFallible<T> {
    fn into_inner(self) -> Vec<T> {
        self.v
    }
}

impl<T> AsRef<Vec<T>> for IntoVecFallible<T> {
    fn as_ref(&self) -> &Vec<T> {
        &self.v
    }
}

impl<T> AsMut<Vec<T>> for IntoVecFallible<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        &mut self.v
    }
}
