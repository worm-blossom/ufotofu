#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::sync::{consumer::TestConsumer, BulkConsumer, Consumer};

fuzz_target!(|data: (TestConsumer<u16, u16, u16>, Box<[u16]>)| {
    let (mut con, items) = data;
    let mut control = con.clone();

    assert_eq!(
        con.bulk_consume_full_slice(&items[..]),
        control.consume_full_slice(&items[..])
    );
    assert_eq!(con, control);
});
