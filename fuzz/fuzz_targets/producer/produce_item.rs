#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{prelude::*, ProduceAtLeastError};

// Generate a TestProducer, and check that produce_item returns the correct value compared to calling `produce` on a clone of the TestConsumer.

fuzz_target!(|data: TestProducer<u16, u16, u16>| {
    pollster::block_on(async {
        let mut pro = data;
        let mut control = pro.clone();

        match pro.produce_item().await {
            Ok(it) => assert_eq!(control.produce().await, Ok(Left(it))),
            Err(ProduceAtLeastError { count, reason }) => {
                assert_eq!(count, 0);

                match reason {
                    Ok(fin) => assert_eq!(control.produce().await, Ok(Right(fin))),
                    Err(err) => assert_eq!(control.produce().await, Err(err)),
                }
            }
        }
    });
});
