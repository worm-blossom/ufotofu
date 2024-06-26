#![no_main]
#![feature(never_type)]
#![feature(maybe_uninit_uninit_array, maybe_uninit_slice)]

use core::cmp::min;
use core::mem::MaybeUninit;

use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::{arbitrary, arbitrary::Arbitrary};

use ufotofu::local_nb;
use ufotofu::local_nb::producer::{ProduceOperations, Scramble, SliceProducer};

#[derive(Debug, Clone, Arbitrary)]
struct TestData {
    producer_buffer: Box<[u8]>,
    operations: ProduceOperations,
    capacity: usize,
    output_buffer_capacity: usize,
}

fn data_is_invalid(data: &TestData) -> bool {
    if data.capacity < 1 || data.capacity > 2048 {
        return true;
    }

    if data.output_buffer_capacity < 1 || data.output_buffer_capacity > 2048 {
        return true;
    }

    false
}

// This allows us to create a `MaybeUninit` boxed slice using a capacity
// value provided by the fuzzer.
fn create_uninit_boxed_slice(slice_capacity: usize) -> Box<[MaybeUninit<u8>]> {
    std::iter::repeat_with(MaybeUninit::uninit)
        .take(slice_capacity)
        .collect()
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
            output_buffer_capacity,
        } = data;

        // Producer.
        let slice_producer = SliceProducer::new(&mut producer_buffer[..]);

        // Scrambler wrapping an inner `slice_producer`.
        let mut scrambled_producer = Scramble::new(slice_producer, operations, capacity);

        // Buffer to fill with items from the producer.
        let mut output_buffer = create_uninit_boxed_slice(output_buffer_capacity);

        // Fill the buffer with items from the producer.
        let (items, _maybe_uninit) =
            local_nb::fill_all(&mut output_buffer, &mut scrambled_producer)
                .await
                .unwrap();

        // Compare the contents of the producer buffer and items buffer.
        let m = min(producer_buffer.len(), items.len());
        assert_eq!(&producer_buffer[..m], &items[..m]);
    })
});
