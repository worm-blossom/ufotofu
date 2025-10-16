#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{prelude::*, ProduceAtLeastError};

// Generate a TestProducer, and check that bulk_overwrite_full_slice returns the correct value and performs the right side-effects compared "normal" production on a clone of the TestConsumer.

fuzz_target!(|data: (TestProducer<u16, u16, u16>, usize)| {
    pollster::block_on(async {
        let (mut pro, mut slice_len) = data;
        slice_len = slice_len.clamp(0, 2048);
        let mut control = pro.clone();

        let mut slice = vec![17; slice_len];

        match pro.bulk_overwrite_full_slice(&mut slice[..]).await {
            Ok(()) => {
                for i in 0..slice_len {
                    assert_eq!(control.produce().await, Ok(Left(slice[i])));
                }
            }
            Err(ProduceAtLeastError { count, reason }) => {
                for i in 0..count {
                    assert_eq!(control.produce().await, Ok(Left(slice[i])));
                }

                match reason {
                    Ok(fin) => assert_eq!(control.produce().await, Ok(Right(fin))),
                    Err(err) => assert_eq!(control.produce().await, Err(err)),
                }
            }
        }
    });
});
