//! Useful functionality for working with producers.
//!
//! ## Obtaining Producers
//!
//! The [FromSlice] producer produces items from a given slice.
//!
//! ## Adaptors
//!
//! The [SyncToLocalNb] adaptor allows you to use a [`sync::Producer`](crate::sync::Producer) as a [`local_nb::Producer`](crate::local_nb::Producer).
//!
//! ## Development Helpers
//!
//! The [Invariant] adaptor wraps any producer and makes it panic during tests when some client code violates the API contracts imposed by the producer traits. In production builds, the wrapper does nothing and compiles away without any overhead. We recommend using this wrapper as an implementation detail of all custom producers; all producers in the ufotofu crate use this wrapper internally as well.
//!
//! The [Scramble] adaptor exists for testing purposes only; it turns a "sensible" pattern of `produce`, `bulk_produce` and `slurp` calls into a much wilder (but still valid) pattern of method calls on the wrapped producer. This is useful for testing corner-cases (you'd rarely write test code that slurps multiple times in succession by hand, for example). To generate the method call patterns, we recommend using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).
//!
//! The [TestProducer] exists for testing code that interacts with a producer; it provides customisable behavior of which items to emit, when to emit the final item or an error, and varies the sizes of bulk buffers it exposes. To generate various configurations, we recommed using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).
//!
//! ## Compatibility
//!
//! To use a [`Producer`] as an [`Stream`], wrap it in a [`ProducerToStream`]. To use an [`Stream`] as a [`Producer`], wrap it in a [`StreamToProducer`]. Requires the `compat_futures` feature.
//!
//! To use a [`BulkProducer`] as a [`futures::AsyncRead`] or [`futures::AsyncBufRead`], wrap it in a [`BulkProducerToAsyncBufRead`]. To use an [`futures::AsyncRead`] as a [`BulkProducer`], wrap it in an [`AsyncReadToBulkProducer`], and to use an [`futures::AsyncBufRead`] as a [`BulkProducer`], wrap it in an [`AsyncBufReadToBulkProducer`]. Requires the `compat_futures` feature.

mod sync_to_local_nb;

pub use sync_to_local_nb::SyncToLocalNb;

pub use crate::common::producer::{FromSlice, Invariant};

#[cfg(feature = "alloc")]
pub use crate::common::producer::FromBoxedSlice;

#[cfg(feature = "dev")]
pub use crate::common::producer::{ProduceOperations, Scramble, TestProducer};

#[cfg(all(feature = "compat_futures", any(feature = "alloc", feature = "std")))]
mod stream;
#[cfg(all(feature = "compat_futures", any(feature = "alloc", feature = "std")))]
pub use stream::*;

// #[cfg(all(feature = "compat_futures", any(feature = "alloc", feature = "std")))]
// mod read;
// #[cfg(all(feature = "compat_futures", any(feature = "alloc", feature = "std")))]
// pub use read::*;
