//! Various types for providing compatibility with core Rust types, standard library types, and — eventually — selected crates.
//!
//! ## [Core](core)
//!
//! Provides [`IntoConsumer`](crate::IntoConsumer) impls for [arrays](core::array) and [slices](core::slice).
//!
//! ## [Alloc](alloc)
//!
//! Provides [`IntoConsumer`](crate::IntoConsumer) impls for [boxed](alloc::boxed::Box) slices and for [`Vec`](alloc::vec::Vec).
//!
//! # [Std](std)
//!
//! Provides [`IntoConsumer`](crate::IntoConsumer) impls for various [collections](std::collections).
//!
//! <br/>Counterpart: the [`producer::compat`](crate::producer::compat) module.

pub mod array;
pub mod slice;

#[cfg(feature = "alloc")]
pub mod vec;

#[cfg(feature = "std")]
pub mod binary_heap;
#[cfg(feature = "std")]
pub mod btree_map;
#[cfg(feature = "std")]
pub mod btree_set;
#[cfg(feature = "std")]
pub mod hash_map;
#[cfg(feature = "std")]
pub mod hash_set;
#[cfg(feature = "std")]
pub mod linked_list;
#[cfg(feature = "std")]
pub mod vec_deque;
