use core::mem::MaybeUninit;
use core::slice;

use std::alloc::{Allocator, Global};
use std::vec::Vec;

use wrapper::Wrapper;

use crate::maybe_uninit_slice_mut;
use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

/// Collects data and can at any point be converted into a `Vec<T>.
pub struct IntoVec<T, A = Global>
where
    A: Allocator,
{
    v: Vec<T, A>,
}

impl<T> Default for IntoVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntoVec<T> {
    pub fn new() -> IntoVec<T> {
        IntoVec { v: Vec::new() }
    }

    pub fn into_vec(self) -> Vec<T> {
        self.v
    }
}

impl<T, A> IntoVec<T, A>
where
    A: Allocator,
{
    pub fn new_in(alloc: A) -> IntoVec<T, A> {
        IntoVec {
            v: Vec::new_in(alloc),
        }
    }
}

impl<T> Consumer for IntoVec<T> {
    type Item = T;
    type Final = ();
    type Error = !;

    fn consume(&mut self, item: T) -> Result<Self::Final, Self::Error> {
        self.v.push(item);

        Ok(())
    }

    fn close(&mut self, _final: Self::Final) -> Result<Self::Final, Self::Error> {
        Ok(())
    }
}

impl<T: Copy> BufferedConsumer for IntoVec<T> {
    fn flush(&mut self) -> Result<Self::Final, Self::Error> {
        Ok(())
    }
}

impl<T: Copy> BulkConsumer for IntoVec<T> {
    fn consumer_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        // Allocate additional capacity to the vector if no empty slots are available.
        if self.v.capacity() == self.v.len() {
            self.v.reserve((self.v.capacity() * 2) + 1);
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

impl<T> Wrapper<Vec<T>> for IntoVec<T> {
    fn into_inner(self) -> Vec<T> {
        self.v
    }
}

impl<T> AsRef<Vec<T>> for IntoVec<T> {
    fn as_ref(&self) -> &Vec<T> {
        &self.v
    }
}

impl<T> AsMut<Vec<T>> for IntoVec<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        &mut self.v
    }
}
