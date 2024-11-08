#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{
    common::errors::ConsumeFullSliceError,
    sync::{consumer::TestConsumer, Consumer},
};

fuzz_target!(|data: (TestConsumer<u16, u16, u16>, Box<[u16]>)| {
    let (mut con, items) = data;

    let expected_err = con.peek_error().unwrap().clone();

    match con.consume_full_slice(&items[..]) {
        Ok(()) => {
            assert!(!con.did_error());
            assert_eq!(con.consumed(), &items[..]);
            assert!(con.final_consumed().is_none());
        }
        Err(ConsumeFullSliceError { consumed, reason }) => {
            assert!(con.did_error());
            assert_eq!(reason, expected_err);
            assert_eq!(con.consumed(), &items[..consumed]);
            assert!(con.final_consumed().is_none());
        }
    }
});
