#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{prelude::*, queues::new_fixed};

// Generate a TestConsumer, turn it into a bulk scrambled, feed a sequence of values into it, and test that the wrapped TestConsumer then stores the same items as a non-scrambled control TestConsumer.

fuzz_target!(|data: (
    TestConsumer<u16, u16, u16>,
    usize,
    Vec<BulkConsumerOperation>,
    Box<[u16]>,
    u16
)| {
    pollster::block_on(async {
        let (con, mut buffer_size, ops, input, fin) = data;
        buffer_size = buffer_size.clamp(1, 4096);

        let mut control = con.clone();
        let mut scrambled = con.bulk_scrambled(new_fixed(buffer_size), ops);

        for x in &input {
            match control.consume(*x).await {
                Err(control_err) => {
                    // Control errored, ensure that the scrambled also errors (flushing if necessary) and behaves equivalently.
                    let scrambled_err = match scrambled.consume(*x).await {
                        Err(err) => err,
                        Ok(()) => scrambled.flush().await.unwrap_err(),
                    };

                    assert_eq!(control_err, scrambled_err);
                    assert_eq!(control, scrambled.into_inner());
                    return;
                }
                Ok(()) => {
                    // No error, also consume in scrambled, then continue with the next item.
                    assert!(scrambled.consume(*x).await.is_ok());
                }
            }
        }

        // Consumed all items without error.
        // Check that closing is equal.
        let res_close = control.close(fin).await;
        assert_eq!(res_close, scrambled.close(fin).await);
        // Finally assert that control and scrambled consumed equal sequences.
        assert_eq!(control, scrambled.into_inner());
    });
});
