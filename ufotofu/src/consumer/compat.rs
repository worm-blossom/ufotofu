//! Various types for providing compatibility with core Rust types, standard library types, and [`futures_lite`] consumer-like abstractions.
//!
//! ## [Core](core)
//!
//! Provides [`IntoConsumer`](crate::IntoConsumer) impls for [arrays](core::array) and [slices](core::slice).
//!
//! ## [Alloc](alloc)
//!
//! Provides [`IntoConsumer`](crate::IntoConsumer) impls for [boxed](alloc::boxed::Box) slices and for [`Vec`](alloc::vec::Vec). Requires the `alloc` feature to be enabled.
//!
//! # [Std](std)
//!
//! Provides [`IntoConsumer`](crate::IntoConsumer) impls for various [collections](std::collections). Requires the `std` feature to be enabled.
//!
//! # [`futures_lite`]
//!
//! Provides an adaptor for using any [`futures_lite::AsyncWrite`] as a [`BulkConsumer`](crate::BulkConsumer). Requires the `compat_futures_io` feature to be enabled.
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

#[cfg(feature = "compat_futures_io")]
pub mod writer;
