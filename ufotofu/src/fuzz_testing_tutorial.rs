//! A tutorial about [fuzz testing](https://rust-fuzz.github.io/book/introduction.html) ufotofu-related code.
//!
//! This tutorial assumes that you already know what fuzz testing is, why you would want to do it, and how to do [structure-aware fuzzing](https://rust-fuzz.github.io/book/cargo-fuzz/structure-aware-fuzzing.html) with the `cargo fuzz` utility.
//!
//! Note that all functionality described here is available only when you enable the `dev` feature of utofotu.
//!
//! ## Obtaining Arbitrary Producers and Consumers
//!
//! Often you might find yourself coding up some functionality that should work for arbitrary producers or consumers. Take, for example the following function which counts how many items a bulk producer produces:
//!
//! ```
//! # use ufotofu::prelude::*;
//! async fn bulk_count<P: BulkProducer>(p: &mut P) -> usize {
//!     let mut counted = 0;
//!
//!     loop {
//!         match p
//!             .expose_items(async |slots| {
//!                 counted += slots.len();
//!                 (slots.len(), ())
//!             })
//!             .await
//!         {
//!             Ok(Left(())) => { /* no-op, continue counting */ }
//!             _ => return counted,
//!         }
//!     }
//! }
//! ```
//!
//! There is an obvious way of checking this function for correctness: randomly generate some items, then create a bulk producer which produces those items, call `bulk_count` on it, and verify that `bulk_count` returns the number of items you created.
//!
//! The [`TestProducer`](crate::producer::TestProducer) type is a testing utility which lets you create (bulk) producers with configurable behaviour. And its [`Arbitrary`](arbitrary::Arbitrary) impl allows a fuzzer to create producers with arbitrary behaviour.
//!
//! The following fuzz test uses `TestProducer` to check the correctness of the `bulk_count` function:
//!
//! ```skip
//! #![no_main]
//!
//! use libfuzzer_sys::fuzz_target;
//! use ufotofu::prelude::*;
//!
//! // Tell the fuzzer to generate `TestProducers` with `u16` Items and `()` as Final and Error.
//! fuzz_target!(|data: TestProducer<u16, (), ()>| {
//!     let mut p = data;
//!
//!     // Obtain the number of items that will be produced.
//!     // (`p.as_slice()` returns a slice of all items which are yet to be produced.)
//!     let len = p.as_slice().len();
//!
//!     // The `pollster` crate lets you run async code in a sync closure.
//!     pollster::block_on(async {
//!         // Call our function on the TestProducer, crash if it behaves incorrectly.
//!         assert_eq!(bulk_count(&mut p).await, len);
//!     });
//! });
//!
//! async fn bulk_count<P: BulkProducer>(p: &mut P) -> usize {
//!     let mut counted = 0;
//!
//!     loop {
//!         match p
//!             .expose_items(async |slots| {
//!                 counted += slots.len();
//!                 (slots.len(), ())
//!             })
//!             .await
//!         {
//!             Ok(Left(())) => { /* no-op, continue counting */ }
//!             _ => return counted,
//!         }
//!     }
//! }
//! ```
//!
//! The counterpart for generating arbitrary (bulk) *consumers* is the [`TestConsumer`](crate::consumer::TestConsumer). The following example shows our fuzz test for the [`pipe`](crate::pipe) function: we generate a random producer *and* a random consumer, and check that after piping all data ends up where it should end up.
//!
//! ```skip
//! #![no_main]
//!
//! use libfuzzer_sys::fuzz_target;
//! use ufotofu::{pipe, prelude::*, PipeError};
//!
//! fuzz_target!(
//!     |data: (TestProducer<u16, u16, u16>, TestConsumer<u16, u16, u16>)| {
//!         pollster::block_on(async {
//!             let (mut pro, mut con) = data;
//!
//!             // Create a copy of the items that will be produced.
//!             let items = pro.as_slice().to_vec();
//!             // Create a copy of the last value to be produced (a `Result<Final, Error>`).
//!             let last = *pro.peek_last().unwrap();
//!
//!             // Pipe the producer into the consumer, then check the outcome.
//!             match pipe(&mut pro, &mut con).await {
//!                 // Neither producer nor consumer emitted an error.
//!                 Ok(()) => {
//!                     // The producer emitted its final item.
//!                     assert!(pro.did_already_emit_last());
//!
//!                     // The slice of consumed items matches the slice of produced items.
//!                     assert_eq!(con.as_slice(), pro.already_produced());
//!
//!                     // The final value of the producer was used to close the consumer.
//!                     assert_eq!(con.peek_final(), Some(&last.unwrap()));
//!                 }
//!                 // The consumer emitted an error before the producer did.
//!                 Err(PipeError::Consumer(_err)) => {
//!                     // The consumer truly did emit its error already.
//!                     assert!(con.did_already_error());
//!
//!                     // The slice of consumed items matches the slice of produced items,
//!                     // except the last one is missing iff the error did not occur on closing.
//!                     if pro.did_already_emit_last() {
//!                         assert_eq!(con.as_slice(), pro.already_produced());
//!                     } else {
//!                         let len_prod = pro.already_produced().len();
//!                         assert_eq!(con.as_slice(), &pro.already_produced()[..len_prod - 1]);
//!                     }
//!                 }
//!                 // The producer emitted an error before the consumer did.
//!                 Err(PipeError::Producer(_err)) => {
//!                     // The producer truly did emit its error already, and the consumer did not.
//!                     assert!(pro.did_already_emit_last());
//!                     assert!(!con.did_already_error());
//!
//!                     // The slice of consumed items matches the slice of produced items.
//!                     assert_eq!(con.as_slice(), &items[..con.as_slice().len()]);
//!                     assert_eq!(pro.as_slice(), &items[con.as_slice().len()..]);
//!                 }
//!             }
//!         });
//!     }
//! );
//! ```
//!
//! Note that the `Arbitrary` impls of `TestProducer` and `TestConsumer` not only create random sequences ans errors, they also create random patterns of how large the slices exposed in bulk processing are, and they will make the trait methods randomly yield back to the executor. The resulting tests are pretty thorough!
