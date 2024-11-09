#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::local_nb::{consumer::TestConsumer, BulkConsumer, Consumer};

fuzz_target!(|data: (TestConsumer<u16, u16, u16>, Box<[u16]>)| {
    smol::block_on(async {
        let (mut con, items) = data;
        let mut control = con.clone();

        assert_eq!(
            con.bulk_consume_full_slice(&items[..]).await,
            control.consume_full_slice(&items[..]).await
        );
        assert_eq!(con, control);
    });
});
