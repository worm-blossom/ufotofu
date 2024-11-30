#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{
    consumer::{BulkConsumerOperation, BulkScrambler, Invariant, TestConsumer},
    Consumer,
};

fuzz_target!(|data: (
    TestConsumer<u16, u16, u16>,
    Vec<BulkConsumerOperation>,
    Box<[u16]>,
    u16
)| {
    pollster::block_on(async {
        let (p1, ops1, input, fin) = data;
        let (p2, ops2) = (p1.clone(), ops1.clone());

        let mut control = BulkScrambler::new(p1, ops1);
        let mut invarianted = Invariant::new(BulkScrambler::new(p2, ops2));

        for x in &input {
            assert_eq!(control.consume(*x).await, invarianted.consume(*x).await);

            if control.as_ref().did_error() {
                break;
            }
        }

        if !control.as_ref().did_error() {
            assert_eq!(control.close(fin).await, invarianted.close(fin).await);
        }

        assert_eq!(control.into_inner(), invarianted.into_inner().into_inner());
    });
});
