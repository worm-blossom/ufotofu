#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{pipe, prelude::*, PipeError};

// Generate a TestProducer and a TestConsumer, pipe the producer into the consumer, and check that the correct number of items (and last values/errors) got moved to the correct place.

fuzz_target!(
    |data: (TestProducer<u16, u16, u16>, TestConsumer<u16, u16, u16>)| {
        pollster::block_on(async {
            let (mut pro, mut con) = data;

            // Create a copy of the items that will be produced.
            let items = pro.as_slice().to_vec();
            // Create a copy of the last value to be produced (a `Result<Final, Error>`).
            let last = *pro.peek_last().unwrap();

            // Pipe the producer into the consumer, then check the outcome.
            match pipe(&mut pro, &mut con).await {
                // Neither producer nor consumer emitted an error.
                Ok(()) => {
                    // The producer emitted its final item.
                    assert!(pro.did_already_emit_last());

                    // The slice of consumed items matches the slice of produced items.
                    assert_eq!(con.as_slice(), pro.already_produced());

                    // The final value of the producer was used to close the consumer.
                    assert_eq!(con.peek_final(), Some(&last.unwrap()));
                }
                // The consumer emitted an error before the producer did.
                Err(PipeError::Consumer(_err)) => {
                    // The consumer truly did emit its error already.
                    assert!(con.did_already_error());

                    // The slice of consumed items matches the slice of produced items,
                    // except the last one is missing iff the error did not occur on closing.
                    if pro.did_already_emit_last() {
                        assert_eq!(con.as_slice(), pro.already_produced());
                    } else {
                        let len_prod = pro.already_produced().len();
                        assert_eq!(con.as_slice(), &pro.already_produced()[..len_prod - 1]);
                    }
                }
                // The producer emitted an error before the consumer did.
                Err(PipeError::Producer(_err)) => {
                    // The producer truly did emit its error already, and the consumer did not.
                    assert!(pro.did_already_emit_last());
                    assert!(!con.did_already_error());

                    // The slice of consumed items matches the slice of produced items.
                    assert_eq!(con.as_slice(), &items[..con.as_slice().len()]);
                    assert_eq!(pro.as_slice(), &items[con.as_slice().len()..]);
                }
            }
        });
    }
);
