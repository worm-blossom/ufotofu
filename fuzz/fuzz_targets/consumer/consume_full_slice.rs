#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{prelude::*, ConsumeAtLeastError};

// Generate TestConsumer and a slice, and check that consume_full_slice returns the correct value and performs the right side-effects compared "normal" item-by-item consumption by a clone of the TestConsumer.

fuzz_target!(|data: (TestConsumer<u16, (), u16>, Box<[u16]>)| {
    pollster::block_on(async {
        let (mut con, slice) = data;
        let slice_len = slice.len();
        let mut control = con.clone();

        match con.consume_full_slice(&slice[..]).await {
            Ok(()) => {
                for i in 0..slice_len {
                    assert_eq!(control.consume(slice[i]).await, Ok(()));
                }
            }
            Err(ConsumeAtLeastError { count, reason }) => {
                for i in 0..count {
                    assert_eq!(control.consume(slice[i]).await, Ok(()));
                }

                assert_eq!(control.consume(slice[count]).await, Err(reason));
            }
        }
    });
});
