//! Useful functionality for working with producers.
//!
//! ## Obtaining Producers
//!
//! The [FromSlice] producer produces items from a given slice.
//!
//! ## Development Helpers
//!
//! The [Invariant] adaptor wraps any producer and makes it panic during tests when some client code violates the API contracts imposed by the producer traits. In production builds, the wrapper does nothing and compiles away without any overhead. We recommend using this wrapper as an implementation detail of all custom producers; all producers in the ufotofu crate use this wrapper internally as well.
//!
//! The [Scramble] adaptor exists for testing producers; it turns a "sensible" pattern of `produce`, `bulk_produce` and `slurp` calls into a much wilder (but still valid) pattern of method calls on the wrapped producer. This is useful for testing corner-cases (you'd rarely write test code that slurps multple times in succession by hand, for example). To generate the method call patterns, we recommed using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).
//!
//! The [TestProducer] exists for testing code that interacts with a producer; it provides customisable behavior of which items to emit, when to emit the final item or an error, and varies the sizes of bulk buffers it exposes. To generate various configurations, we recommed using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).

pub use crate::common::producer::{FromSlice, Invariant};

#[cfg(feature = "alloc")]
pub use crate::common::producer::FromBoxedSlice;

#[cfg(feature = "dev")]
pub use crate::common::producer::{ProduceOperations, Scramble, TestProducer};
