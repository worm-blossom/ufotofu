//! Useful functionality for working with consumers.
//!
//! ## Obtaining Consumers
//!
//! The [`IntoVec`] consumer consumes an arbitrary number of items and can be turned into a [`Vec`](std::vec::Vec) of all consumed items.
//!
//! ## Adaptors
//!
//! The [`MapItem`] adaptor wraps any consumer and maps the items it receives with a function.
//!
//! ## Development Helpers
//!
//! The [Invariant] adaptor wraps any consumer and makes it panic during tests when some client code violates the API contracts imposed by the consumer traits. In production builds, the wrapper does nothing and compiles away without any overhead. We recommend using this wrapper as an implementation detail of all custom consumers; all consumers in the ufotofu crate use this wrapper internally.
//!
//! The [TestConsumer] exists for testing code that interacts with arbitrary consumers; it provides customisable behavior of how many items to consume before emitting a configurable error, and varies the sizes of bulk buffers it exposes. To generate various configurations, we recommed using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).
//!
//! The [BulkScrambler] exists for testing specific [`BulkConsumer`](ufotofu::BulkConsumer)s by exercising various interleavings of `consume`, `flush`, and `consume_slots` calls. To generate various configurations, we recommed using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).

#[macro_use]
mod macros;

// mod into_slice;
// pub use into_slice::IntoSlice_ as IntoSlice;

#[cfg(feature = "alloc")]
mod into_vec;
#[cfg(feature = "alloc")]
pub use into_vec::IntoVec_ as IntoVec;

mod map_item;
pub use map_item::MapItem;

#[cfg(test)]
mod invariant;
#[cfg(not(test))]
mod invariant_noop;
#[cfg(test)]
pub use invariant::Invariant;
#[cfg(not(test))]
pub use invariant_noop::Invariant;

#[cfg(feature = "dev")]
mod bulk_scrambler;
#[cfg(feature = "dev")]
pub use bulk_scrambler::{BulkConsumerOperation, BulkScrambler_ as BulkScrambler};

#[cfg(all(feature = "dev", feature = "alloc"))]
mod test_consumer;
#[cfg(all(feature = "dev", feature = "alloc"))]
pub use test_consumer::{TestConsumerBuilder, TestConsumer_ as TestConsumer};
