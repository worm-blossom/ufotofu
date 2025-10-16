#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::prelude::*;

// Generate two TestProducer, and check that ProducerExt::equals agrees with the Eq impl of the TestProducer.

fuzz_target!(
    |data: (TestProducer<u16, u16, u16>, TestProducer<u16, u16, u16>)| {
        pollster::block_on(async {
            let (mut p1, mut p2) = data;

            assert_eq!(p1 == p2, p1.equals_dbg(&mut p2).await);
        });
    }
);
