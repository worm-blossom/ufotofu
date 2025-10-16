#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{prelude::*, queues::new_fixed};

// Generate a TestConsumer, buffer it with the buffered method, turn *that* into a bulk scrambled, feed a sequence of values into it, and test that the wrapped buffered TestConsumer forwarded the same items to the underlying TestConsumer as a control TestConsumer.

fuzz_target!(|data: (
    TestConsumer<u16, u16, u16>,
    usize,
    usize,
    Vec<BulkConsumerOperation>,
    Box<[u16]>,
    u16
)| {
    pollster::block_on(async {
        let (con, mut buffered_buffer_size, mut scrambler_buffer_size, ops, input, fin) = data;
        // Clamp the sizes of internal buffers to ensure non-empty, not-too-large queues.
        buffered_buffer_size = buffered_buffer_size.clamp(1, 4096);
        scrambler_buffer_size = scrambler_buffer_size.clamp(1, 4096);

        // A plain consumer; our buffered, scrambled version should behave just like this one.
        let mut control = con.clone();

        // The consumer we want to test. We will feed equal sequences to both the control consumer
        // and this one, asserting that the consumer to which we added buffering ultimately
        // receives the same data as the non-buffered one.
        let consumer_under_test = con.buffered(new_fixed(buffered_buffer_size));

        // And we scramble the buffered consumer, to exercise all usage patterns.
        let mut scrambled =
            consumer_under_test.bulk_scrambled(new_fixed(scrambler_buffer_size), ops);

        for x in &input {
            match control.consume(*x).await {
                Ok(()) => {
                    // Whenever the control consumer successfully consumes an item, we also feed
                    // that item into the buffered consumer.
                    assert!(scrambled.consume(*x).await.is_ok());
                }
                Err(control_err) => {
                    // When the control consumer errors, we want to ensure that the buffered
                    // consumer also errors. Due to buffering, we might need a flush to trigger
                    // the error.
                    let scrambled_err = match scrambled.consume(*x).await {
                        Err(err) => err,
                        Ok(()) => scrambled.flush().await.expect_err(
                            "control consumer errored, so the buffered consumer must also error",
                            // unless the buffered consumer is buffy, which is what we are testing
                        ),
                    };

                    // After retrieving the error of the buffered consumer, we check that it is
                    // the correct error, and that before the error, equal sequences were
                    // consumed by both the control and the wrapped consumer.
                    assert_eq!(control_err, scrambled_err);
                    assert_eq!(control, scrambled.into_inner().into_inner());
                    return;
                }
            }
        }

        // We consumed all items without error.
        // Check that closing yields equal results.
        assert_eq!(control.close(fin).await, scrambled.close(fin).await);

        // Finally assert that the scrambled and buffered consumer ultimately received equal
        // sequences of values.
        assert_eq!(control, scrambled.into_inner().into_inner());
    });
});
