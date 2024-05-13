#![no_main]
use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::{arbitrary, arbitrary::Arbitrary};

use core::cmp::min;

use wrapper::Wrapper;

use ufotofu::sync::consumer::Cursor as ConsumerCursor;
use ufotofu::sync::consumer::{ConsumeOperations, Scramble, ScrambleError};
use ufotofu::sync::producer::Cursor as ProducerCursor;
use ufotofu::sync::{self, BufferedConsumer};

fn data_is_invalid(data: TestData) -> bool {
    if data.consumer_buf.len() < data.producer_buf.len() {
        return true;
    }

    if data.capacity_a < 1 || data.capacity_a > 2048 {
        return true;
    }

    if data.capacity_b < 1 || data.capacity_b > 2048 {
        return true;
    }

    false
}

#[derive(Debug, Clone, Arbitrary)]
struct TestData {
    consumer_buf: Box<[u8]>,
    producer_buf: Box<[u8]>,
    operations_a: ConsumeOperations,
    operations_b: ConsumeOperations,
    capacity_a: usize,
    capacity_b: usize,
}

fuzz_target!(|data: TestData| {
    if data_is_invalid(data.clone()) {
        return;
    }

    let TestData {
        mut consumer_buf,
        producer_buf,
        operations_a,
        operations_b,
        capacity_a,
        capacity_b,
    } = data;

    let cursor = ConsumerCursor::new(&mut consumer_buf[..]);

    let mut o = ProducerCursor::new(&producer_buf[..]);
    let mut i = Scramble::new(
        Scramble::new(cursor, operations_b, capacity_b),
        operations_a,
        capacity_a,
    );

    let _ = sync::bulk_pipe::<_, _, ScrambleError>(&mut o, &mut i);
    let _ = i.flush();

    // Compare the producer and consumer cursors.
    let i = i.into_inner().into_inner();
    let m = min(o.as_ref().len(), i.as_ref().len());
    assert_eq!(&i.as_ref()[..m], &o.as_ref()[..m]);
});
