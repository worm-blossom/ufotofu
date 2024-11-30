#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{
    consumer::{BulkConsumerOperation, BulkScrambler, TestConsumer},
    BufferedConsumer, Consumer,
};

fuzz_target!(|data: (
    TestConsumer<u16, u16, u16>,
    Vec<BulkConsumerOperation>,
    Box<[u16]>,
    u16
)| {
    pollster::block_on(async {
        let (mut control, ops, input, fin) = data;
        let mut scrambler = BulkScrambler::new(control.clone(), ops);

        for x in &input {
            match control.consume(*x).await {
                Err(control_err) => {
                    // Control errored, ensure that the scrambler also errors (flushing if necessary) and behaves equivalently.
                    let scrambler_err = match scrambler.consume(*x).await {
                        Err(err) => err,
                        Ok(()) => scrambler.flush().await.unwrap_err(),
                    };

                    assert_eq!(control_err, scrambler_err);
                    assert_eq!(control, scrambler.into_inner());
                    return;
                }
                Ok(()) => {
                    // No error, also consume in scrambler, then continue with the next item.
                    assert!(scrambler.consume(*x).await.is_ok());
                }
            }
        }

        // Consumed all items without error.
        // Check that closing is equal.
        let res_close = control.close(fin).await;
        assert_eq!(res_close, scrambler.close(fin).await);
        // Finally assert that control and scrambler consumed equal sequences.
        assert_eq!(control, scrambler.into_inner());
    });
});
