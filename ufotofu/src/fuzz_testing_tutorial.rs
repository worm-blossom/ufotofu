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
//! ```no_run
//! #![no_main]
//! use libfuzzer_sys::fuzz_target;
//! use ufotofu::prelude::*;
//!
//! # #[cfg(feature = "dev")] {
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
//! #
//! # async fn bulk_count<P: BulkProducer>(p: &mut P) -> usize {
//! #     let mut counted = 0;
//! #
//! #     loop {
//! #         match p
//! #             .expose_items(async |slots| {
//! #                 counted += slots.len();
//! #                 (slots.len(), ())
//! #             })
//! #             .await
//! #         {
//! #             Ok(Left(())) => { /* no-op, continue counting */ }
//! #             _ => return counted,
//! #         }
//! #     }
//! # }
//! # }
//! ```
//!
//! The counterpart for generating arbitrary (bulk) *consumers* is the [`TestConsumer`](crate::consumer::TestConsumer). The following example shows our fuzz test for the [`pipe`](crate::pipe) function: we generate a random producer *and* a random consumer, and check that after piping all data ends up where it should end up.
//!
//! ```no_run
//! #![no_main]
//! use libfuzzer_sys::fuzz_target;
//! use ufotofu::{pipe, prelude::*, PipeError};
//!
//! # #[cfg(feature = "dev")] {
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
//! # }
//! ```
//!
//! Note that the `Arbitrary` impls of `TestProducer` and `TestConsumer` not only create random sequences ans errors, they also create random patterns of how large the slices exposed in bulk processing are, and they will make the trait methods randomly yield back to the executor. The resulting tests are pretty thorough!
//!
//! ## Testing Producers and Consumers
//!
//! Testing code that works with arbitrary producers and consumers is one thing, but often you have implemented a concrete bulk producer or bulk cosumer yourself, and would like to verify its correctness. In particular, it should function correctly even under the most bizarre combinations of individual operations, bulk operations, and flush/slurp calls. This is where [`BulkProducerExt::bulk_scrambled`](crate::producer::BulkProducerExt::bulk_scrambled) and [`BulkConsumerExt::bulk_scrambled`](crate::consumer::BulkConsumerExt::bulk_scrambled) enter the picture.
//!
//! These methods allow to you to wrap any bulk processor to obtain a new one. The new processor produces or consumes the exact same sequences, but it interacts with the wrapped producer according to a set pattern. And by letting a fuzzer generate this pattern, we can have it explore all sorts of interesting corner cases.
//!
//! Take, as a silly example, the following bulk explorer for repeating an item exactly 17 times:
//!
//! ```no_run
//! use ufotofu::prelude::*;
//!
//! struct SeventeenTimes<T> {
//!     already_emitted: usize,
//!     items: [T; 17],
//! }
//!
//! fn seventeen_times<T: Clone>(item: T) -> SeventeenTimes<T> {
//!     SeventeenTimes {
//!         already_emitted: 0,
//!         items: core::array::from_fn(|_| item.clone()),
//!     }
//! }
//!
//! impl<T: Clone> Producer for SeventeenTimes<T> {
//!     type Item = T;
//!     type Final = ();
//!     type Error = Infallible;
//!
//!     async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
//!         if self.already_emitted == 17 {
//!             return Ok(Right(()));
//!         } else {
//!             self.already_emitted += 1;
//!             return Ok(Left(self.items[0].clone()));
//!         }
//!     }
//!
//!     async fn slurp(&mut self) -> Result<(), Self::Error> {
//!         return Ok(());
//!     }
//! }
//!
//! impl<T: Clone> BulkProducer for SeventeenTimes<T> {
//!     async fn expose_items<F, R>(&mut self, f: F) -> Result<Either<R, Self::Final>, Self::Error>
//!     where
//!         F: AsyncFnOnce(&[Self::Item]) -> (usize, R),
//!     {
//!         if self.already_emitted == 17 {
//!             return Ok(Right(()));
//!         } else {
//!             let (amount, ret) = f(&self.items[self.already_emitted..]).await;
//!             self.already_emitted += amount;
//!             return Ok(Left(ret));
//!         }
//!     }
//! }
//! ```
//!
//! The following short fuzz test ensures that it operates correctly:
//!
//! ```no_run
//! #![no_main]
//! use libfuzzer_sys::fuzz_target;
//! use ufotofu::{prelude::*, queues::new_fixed};
//!
//! # #[cfg(feature = "dev")] {
//! // Tell the fuzzer to generate a random item to repreat, and
//! // the random data we need to create a scrambled version of `SeventeenTimes`.
//! fuzz_target!(|data: (u16, usize, Vec<BulkProducerOperation>)| {
//!     pollster::block_on(async {
//!         let (item, mut buffer_size, ops) = data;
//!         // Ensure the scrambler uses a non-empty but not memory-shattering internal buffer.
//!         buffer_size = buffer_size.clamp(1, 4096);
//!
//!         // Create the producer-under-test...
//!         let pro = seventeen_times(item);
//!         // ...and wrap it in a scrambler.
//!         let mut scrambled = pro.bulk_scrambled(new_fixed(buffer_size), ops);
//!
//!         // Interact with the scrambled version in a simple way, the scrambler then
//!         // exercises the producer-under-test according to the access pattern supplied
//!         // by the fuzzer.
//!         for _ in 0..17 {
//!             assert_eq!(scrambled.produce().await, Ok(Left(item)));
//!         }
//!         assert_eq!(scrambled.produce().await, Ok(Right(())));
//!     });
//! });
//! # struct SeventeenTimes<T> {
//! #     already_emitted: usize,
//! #     items: [T; 17],
//! # }
//! #
//! # fn seventeen_times<T: Clone>(item: T) -> SeventeenTimes<T> {
//! #     SeventeenTimes {
//! #         already_emitted: 0,
//! #         items: core::array::from_fn(|_| item.clone()),
//! #     }
//! # }
//! #
//! # impl<T: Clone> Producer for SeventeenTimes<T> {
//! #     type Item = T;
//! #     type Final = ();
//! #     type Error = Infallible;
//! #
//! #      async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
//! #         if self.already_emitted == 17 {
//! #             return Ok(Right(()));
//! #         } else {
//! #             self.already_emitted += 1;
//! #             return Ok(Left(self.items[0].clone()));
//! #         }
//! #     }
//! #
//! #     async fn slurp(&mut self) -> Result<(), Self::Error> {
//! #         return Ok(());
//! #     }
//! # }
//! #
//! # impl<T: Clone> BulkProducer for SeventeenTimes<T> {
//! #     async fn expose_items<F, R>(&mut self, f: F) -> Result<Either<R, Self::Final>, Self::Error>
//! #     where
//! #         F: AsyncFnOnce(&[Self::Item]) -> (usize, R),
//! #     {
//! #         if self.already_emitted == 17 {
//! #             return Ok(Right(()));
//! #         } else {
//! #             let (amount, ret) = f(&self.items[self.already_emitted..]).await;
//! #             self.already_emitted += amount;
//! #             return Ok(Left(ret));
//! #         }
//! #     }
//! # }
//! # }
//! ```
//!
//! Although the test code simply calls [`produce`](crate::Producer::produce) repeatedly, the wrapped `SeventeenTimes` producer is subjected to whichever bizarre access pattern the fuzzer has dreamed up.
//!
//! The following example demonstrates not only scrambling of bulk *consumers*, but it also shows a common technique for fuzzing adaptors: we employ both a `TestConsumer` (as the consumer being adapted) and scrambling (to exercise the adaptor under arbitrary usage patterns). Note that the scrambler for bulk consumers may internally buffer items before forwarding them to the wrapped consumer, so you may have to `flush` it before seeing the final results (demonstrated in the consumer-error-handling case of the example).
//!
//! ```no_run
//! #![no_main]
//! use libfuzzer_sys::fuzz_target;
//! use ufotofu::{prelude::*, queues::new_fixed};
//!
//! # #[cfg(feature = "dev")] {
//! // Generate a TestConsumer, buffer it with the bulk_buffered method, turn *that* into a
//! // bulk scrambled, feed a sequence of values into it, and test that the wrapped buffered
//! // TestConsumer forwarded the same items to the underlying TestConsumer as a control
//! // consumer received.
//! fuzz_target!(|data: (
//!     TestConsumer<u16, u16, u16>,
//!     usize,
//!     usize,
//!     Vec<BulkConsumerOperation>,
//!     Box<[u16]>,
//!     u16
//! )| {
//!     pollster::block_on(async {
//!         let (con, mut buffered_buffer_size, mut scrambler_buffer_size, ops, input, fin) = data;
//!         // Clamp the sizes of internal buffers to ensure non-empty, not-too-large queues.
//!         buffered_buffer_size = buffered_buffer_size.clamp(1, 4096);
//!         scrambler_buffer_size = scrambler_buffer_size.clamp(1, 4096);
//!
//!         // A plain consumer; our buffered, scrambled version should behave just like this one.
//!         let mut control = con.clone();
//!
//!         // The consumer we want to test. We will feed equal sequences to both the control
//!         // consumer and this one, asserting that the consumer to which we added buffering
//!         // ultimately receives the same data as the non-buffered one.
//!         let consumer_under_test = con.bulk_buffered(new_fixed(buffered_buffer_size));
//!
//!         // And we scramble the buffered consumer, to exercise all usage patterns.
//!         let mut scrambled =
//!             consumer_under_test.bulk_scrambled(new_fixed(scrambler_buffer_size), ops);
//!
//!         for x in &input {
//!             match control.consume(*x).await {
//!                 Ok(()) => {
//!                     // Whenever the control consumer successfully consumes an item, we also
//!                     // feed that item into the buffered consumer.
//!                     assert!(scrambled.consume(*x).await.is_ok());
//!                 }
//!                 Err(control_err) => {
//!                     // When the control consumer errors, we want to ensure that the buffered
//!                     // consumer also errors. Due to buffering, we might need a flush to trigger
//!                     // the error.
//!                     let scrambled_err = match scrambled.consume(*x).await {
//!                         Err(err) => err,
//!                         Ok(()) => scrambled.flush().await.expect_err(
//!                             "control consumer errored, so the buffered consumer must also error",
//!                             // (or the buffered consumer is buggy, which is what test for)
//!                         ),
//!                     };
//!
//!                     // After retrieving the error of the buffered consumer, we check that it is
//!                     // the correct error, and that before the error, equal sequences were
//!                     // consumed by both the control and the wrapped consumer.
//!                     assert_eq!(control_err, scrambled_err);
//!                     assert_eq!(control, scrambled.into_inner().into_inner());
//!                     return;
//!                 }
//!             }
//!         }
//!
//!         // We consumed all items without error.
//!         // Check that closing yields equal results.
//!         assert_eq!(control.close(fin).await, scrambled.close(fin).await);
//!
//!         // Finally assert that the scrambled and buffered consumer ultimately received equal
//!         // sequences of values.
//!         assert_eq!(control, scrambled.into_inner().into_inner());
//!     });
//! });
//! # }
//! ```
//!
//! ## Other
//!
//! The [`ProducerExt::equals`](crate::producer::ProducerExt::equals) method is convenient for asserting that some tested producer emits the same sequence as a control producer. Its boolean output is rather useless for debugging, however. The [`ProducerExt::equals_dbg`](crate::producer::ProducerExt::equals_dbg) does the same as [`ProducerExt::equals`](crate::producer::ProducerExt::equals), but additionally logs the values `produced` by both producers.
