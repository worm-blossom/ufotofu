#![feature(maybe_uninit_write_slice)]

use core::mem::MaybeUninit;

//pub mod nb;
pub mod sync;

pub(crate) fn maybe_uninit_slice_mut<'a, T>(s: &'a mut [T]) -> &'a mut [MaybeUninit<T>] {
    let ptr = s.as_mut_ptr().cast::<MaybeUninit<T>>();
    unsafe { core::slice::from_raw_parts_mut(ptr, s.len()) }
}
