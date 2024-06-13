#![no_std]
#![feature(maybe_uninit_write_slice)]
#![feature(maybe_uninit_uninit_array)]
#![feature(never_type)]
#![feature(allocator_api)]
#![feature(vec_push_within_capacity)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

use core::mem::MaybeUninit;

pub mod local_nb;
pub mod nb;
pub mod sync;

pub(crate) fn maybe_uninit_slice_mut<T>(s: &mut [T]) -> &mut [MaybeUninit<T>] {
    let ptr = s.as_mut_ptr().cast::<MaybeUninit<T>>();
    unsafe { core::slice::from_raw_parts_mut(ptr, s.len()) }
}
