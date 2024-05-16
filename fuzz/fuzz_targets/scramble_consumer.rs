#![no_main]
use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::{arbitrary, arbitrary::Arbitrary};

use core::cmp::min;

use wrapper::Wrapper;

use ufotofu::sync::consumer::{ConsumeOperations, IntoVec, Scramble, ScrambleError};
use ufotofu::sync::producer::Cursor;
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
    outer_operations: ConsumeOperations,
    inner_operations: ConsumeOperations,
    outer_capacity: usize,
    inner_capacity: usize,
}

fuzz_target!(|data: TestData| {
    if data_is_invalid(&data) {
        return;
    }

    let TestData {
        producer_buffer,
        outer_operations,
        inner_operations,
        outer_capacity,
        inner_capacity,
    } = data;

    // Consumer.
    let into_vec = IntoVec::new();

    // Producer.
    let mut o = Cursor::new(&producer_buffer[..]);

    // Scrambler wrapping a scrambler with an inner `into_vec` consumer.
    let mut i = Scramble::new(
        Scramble::new(into_vec, inner_operations, inner_capacity),
        outer_operations,
        outer_capacity,
    );

    let _ = sync::bulk_pipe::<_, _, ScrambleError>(&mut o, &mut i);
    let _ = i.flush();

    // Access the inner consumer (`into_vec`).
    let i = i.into_inner().into_inner();

    // Compare the contents of the consumer and producer.
    let m = min(o.as_ref().len(), i.as_ref().len());
    assert_eq!(&i.as_ref()[..m], &o.as_ref()[..m]);
});
