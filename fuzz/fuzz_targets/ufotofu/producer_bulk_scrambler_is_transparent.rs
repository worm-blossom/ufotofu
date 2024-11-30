#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{
    producer::{BulkProducerOperation, BulkScrambler, TestProducer},
    Producer,
};

fuzz_target!(
    |data: (TestProducer<u16, u16, u16>, Vec<BulkProducerOperation>,)| {
        pollster::block_on(async {
            let (mut control, ops) = data;
            let mut scrambler = BulkScrambler::new(control.clone(), ops);

            while !control.did_already_emit_last() {
                assert_eq!(control.produce().await, scrambler.produce().await);
            }
        });
    }
);
