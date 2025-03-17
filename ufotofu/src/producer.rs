//! Useful functionality for working with producers.
//!
//! ## Obtaining Producers
//!
//! The [`FromSlice`] producer produces the items of a slice.
//!
//! The [`FromBoxedSlice`] producer takes ownership of a boxed slice (or vector) and produces its items.
//!
//! ## Adaptors
//!
//! The [`MapItem`] adaptor wraps any producer and maps its emitted items with a function.
//!
//! ## Development Helpers
//!
//! The [Invariant] adaptor wraps any producer and makes it panic during tests when some client code violates the API contracts imposed by the producer traits. In production builds, the wrapper does nothing and compiles away without any overhead. We recommend using this wrapper as an implementation detail of all custom producers; all producers in the ufotofu crate use this wrapper internally.
//!
//! The [TestProducer] exists for testing code that interacts with arbitrary producers; it provides customisable behavior of which items to emit, when to emit the final item or an error, and varies the sizes of bulk buffers it exposes. To generate various configurations, we recommed using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).
//!
//! The [BulkScrambler] exists for testing specific [`BulkProducer`](ufotofu::BulkProducer)s by exercising various interleavings of `produce`, `slurp`, and `expose_items` calls. To generate various configurations, we recommed using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).

#[macro_use]
mod macros;

mod from_slice;
pub use from_slice::FromSlice_ as FromSlice;

mod from_boxed_slice;
pub use from_boxed_slice::FromBoxedSlice_ as FromBoxedSlice;

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
pub use bulk_scrambler::{BulkProducerOperation, BulkScrambler_ as BulkScrambler};

#[cfg(all(feature = "dev", feature = "alloc"))]
mod test_producer;
#[cfg(all(feature = "dev", feature = "alloc"))]
pub use test_producer::{TestProducerBuilder, TestProducer_ as TestProducer};
