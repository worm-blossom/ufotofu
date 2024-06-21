#![no_main]
#![feature(never_type)]

use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::{arbitrary, arbitrary::Arbitrary};

use core::cmp::min;

use wrapper::Wrapper;

use ufotofu::local_nb::consumer::IntoVec;
use ufotofu::local_nb::producer::{ProduceOperations, Scramble, SliceProducer};
use ufotofu::local_nb::{self, BufferedConsumer};

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
    smol::block_on(async {
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
        let slice_producer = SliceProducer::new(&mut producer_buffer[..]);

        // Consumer.
        let mut i = IntoVec::new();

        // Scrambler wrapping a scrambler with an inner `slice_producer`.
        let mut o = Scramble::new(
            Scramble::new(slice_producer, inner_operations, inner_capacity),
            outer_operations,
            outer_capacity,
        );

        let _ = local_nb::bulk_pipe(&mut o, &mut i).await;
        let _ = i.flush().await;

        // Access the inner producer (`slice_producer`).
        let o = o.into_inner().into_inner();

        // Compare the contents of the producer and consumer.
        let m = min(o.as_ref().len(), i.as_ref().len());
        assert_eq!(&i.as_ref()[..m], &o.as_ref()[..m]);
    })
});
