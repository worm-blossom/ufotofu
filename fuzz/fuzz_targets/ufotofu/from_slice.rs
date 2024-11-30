#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{
    consumer::IntoVec,
    pipe,
    producer::{BulkProducerOperation, BulkScrambler, FromSlice},
};

fuzz_target!(|data: (Box<[u8]>, Vec<BulkProducerOperation>)| {
    pollster::block_on(async {
        let (input, ops) = data;

        let mut p = BulkScrambler::new(FromSlice::new(&input[..]), ops);

        let mut into_vec = IntoVec::new();

        assert!(pipe(&mut p, &mut into_vec).await.is_ok());
        assert_eq!(&input[..], &into_vec.into_vec()[..]);
    });
});
