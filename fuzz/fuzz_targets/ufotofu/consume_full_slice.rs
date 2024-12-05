#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{consumer::TestConsumer, ConsumeAtLeastError, Consumer};

fuzz_target!(|data: (TestConsumer<u16, u16, u16>, Box<[u16]>)| {
    pollster::block_on(async {
        let (mut con, items) = data;

        let expected_err = *con.peek_error().unwrap();

        match con.consume_full_slice(&items[..]).await {
            Ok(()) => {
                assert!(!con.did_error());
                assert_eq!(con.consumed(), &items[..]);
                assert!(con.final_consumed().is_none());
            }
            Err(ConsumeAtLeastError { count, reason }) => {
                assert!(con.did_error());
                assert_eq!(reason, expected_err);
                assert_eq!(con.consumed(), &items[..count]);
                assert!(con.final_consumed().is_none());
            }
        }
    })
});
