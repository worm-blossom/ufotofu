//! Various types for providing compatibility with core Rust types, standard library types, and — eventually — selected crates.
//!
//! ## [Core](core)
//!
//! Lets you convert iterators into producers with the [`iterator_to_producer`] and [`infinite_iterator_to_producer`] functions.
//!
//! Provides [`IntoProducer`](IntoProducer) impls for [arrays](core::array) and [slices](core::slice).
//!
//! ## [Alloc](alloc)
//!
//! Provides [`IntoProducer`](IntoProducer) impls for [boxed](alloc::boxed::Box) slices and for [`Vec`].
//!
//! # [Std](std)
//!
//! Provides [`IntoProducer`](IntoProducer) impls for various [collections](std::collections).
//!
//! <br/>Counterpart: the [`consumer::compat`] module.

mod iterator_to_producer;
pub use iterator_to_producer::*;

mod infinite_iterator_to_producer;
pub use infinite_iterator_to_producer::*;

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
