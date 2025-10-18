#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::prelude::*;

// Generate a TestConsumer, clone it, and then check that a bulk_consume call agrees with the corresponding individual consume calls on the clone.

fuzz_target!(|data: (TestConsumer<u16, u16, u16>, Box<[u16]>)| {
    pollster::block_on(async {
        let (mut con, slice) = data;
        let slice_len = slice.len();
        let mut control = con.clone();

        match con.bulk_consume(&slice[..]).await {
            Ok(amount) => {
                assert!(slice_len == 0 || amount > 0);

                for i in 0..amount {
                    assert_eq!(control.consume(slice[i]).await, Ok(()));
                }
            }
            Err(err) => {
                assert_eq!(control.consume(99).await, Err(err));
            }
        }
    });
});
