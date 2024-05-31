#![no_main]
#![feature(never_type)]

use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::{arbitrary, arbitrary::Arbitrary};

use core::cmp::min;

use wrapper::Wrapper;

use ufotofu::sync::consumer::IntoVec;
use ufotofu::sync::producer::{Cursor, ProduceOperations, Scramble};
use ufotofu::sync::{self, BufferedConsumer};

fn data_is_invalid(data: &TestData) -> bool {
    if data.outer_capacity < 1 || data.outer_capacity > 2048 {
        return true;
    }

    if data.inner_capacity < 1 || data.inner_capacity > 2048 {
        return true;
    }

    false
}

#[derive(Debug, Clone, Arbitrary)]
struct TestData {
    producer_buffer: Box<[u8]>,
    outer_operations: ProduceOperations,
    inner_operations: ProduceOperations,
    outer_capacity: usize,
    inner_capacity: usize,
}

fuzz_target!(|data: TestData| {
    if data_is_invalid(&data) {
        return;
    }

    let TestData {
        mut producer_buffer,
        outer_operations,
        inner_operations,
        outer_capacity,
        inner_capacity,
    } = data;

    // Producer.
    let cursor = Cursor::new(&mut producer_buffer[..]);

    // Consumer.
    let mut i = IntoVec::new();

    // Scrambler wrapping a scrambler with an inner `cursor` producer.
    let mut o = Scramble::new(
        Scramble::new(cursor, inner_operations, inner_capacity),
        outer_operations,
        outer_capacity,
    );

    let _ = sync::bulk_pipe::<_, _, !>(&mut o, &mut i);
    let _ = i.flush();

    // Access the inner producer (`cursor`).
    let o = o.into_inner().into_inner();

    // Compare the contents of the producer and consumer.
    let m = min(o.as_ref().len(), i.as_ref().len());
    assert_eq!(&i.as_ref()[..m], &o.as_ref()[..m]);
});
