#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{
    producer::{BulkProducerOperation, BulkScrambler, Invariant, TestProducer},
    Producer,
};

fuzz_target!(
    |data: (TestProducer<u16, u16, u16>, Vec<BulkProducerOperation>,)| {
        pollster::block_on(async {
            let (p1, ops1) = data;
            let (p2, ops2) = (p1.clone(), ops1.clone());

            let mut control = BulkScrambler::new(p1, ops1);
            let mut invarianted = Invariant::new(BulkScrambler::new(p2, ops2));

            while !control.as_ref().did_already_emit_last() {
                assert_eq!(control.produce().await, invarianted.produce().await);
            }
        });
    }
);
