#![no_main]
#![feature(never_type)]
#![feature(maybe_uninit_uninit_array, maybe_uninit_slice)]

use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::{arbitrary, arbitrary::Arbitrary};

use core::cmp::min;
use core::mem::MaybeUninit;

use wrapper::Wrapper;

use ufotofu::local_nb;
use ufotofu::local_nb::producer::{ProduceOperations, Scramble, SliceProducer};

fn data_is_invalid(data: &TestData) -> bool {
    if data.capacity < 1 || data.capacity > 2048 {
        return true;
    }

    false
}

#[derive(Debug, Clone, Arbitrary)]
struct TestData {
    producer_buffer: Box<[u8]>,
    operations: ProduceOperations,
    capacity: usize,
}

fuzz_target!(|data: TestData| {
    smol::block_on(async {
        if data_is_invalid(&data) {
            return;
        }

        let TestData {
            mut producer_buffer,
            operations,
            capacity,
        } = data;

        // Producer.
        let slice_producer = SliceProducer::new(&mut producer_buffer[..]);

        // Scrambler wrapping an inner `slice_producer`.
        let mut o = Scramble::new(slice_producer, operations, capacity);

        // Buffer to fill with items from the producer.
        let mut buffer: [MaybeUninit<u8>; 2048] = MaybeUninit::uninit_array();

        // Fill the buffer with items from the producer.
        let (items, _maybe_uninit_) = local_nb::fill_all(&mut buffer, &mut o).await.unwrap();

        // Access the inner producer (`slice_producer`).
        let o = o.into_inner();

        // Compare the contents of the items buffer and producer.
        let m = min(items.len(), o.as_ref().len());
        assert_eq!(&items[..m], &o.as_ref()[..m]);
    })
});
