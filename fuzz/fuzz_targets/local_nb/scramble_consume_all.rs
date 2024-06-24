#![no_main]
#![feature(never_type)]

use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::{arbitrary, arbitrary::Arbitrary};

use core::cmp::min;

use wrapper::Wrapper;

use ufotofu::local_nb::consumer::{ConsumeOperations, IntoVec, Scramble};
use ufotofu::local_nb::{self, BufferedConsumer};

fn data_is_invalid(data: &TestData) -> bool {
    if data.capacity < 1 || data.capacity > 2048 {
        return true;
    }

    false
}

#[derive(Debug, Clone, Arbitrary)]
struct TestData {
    items: Box<[u8]>,
    operations: ConsumeOperations,
    capacity: usize,
}

fuzz_target!(|data: TestData| {
    smol::block_on(async {
        if data_is_invalid(&data) {
            return;
        }

        let TestData {
            items,
            operations,
            capacity,
        } = data;

        // Consumer.
        let into_vec = IntoVec::new();

        // Scrambler wrapping an inner `into_vec`.
        let mut i = Scramble::new(into_vec, operations, capacity);

        // Consume all items from the `items` buffer.
        let _ = local_nb::consume_all(&items, &mut i).await;
        let _ = i.flush().await;

        // Access the inner consumer (`into_vec`).
        let i = i.into_inner();

        // Compare the contents of the items buffer and consumer.
        let m = min(items.len(), i.as_ref().len());
        assert_eq!(&items[..m], &i.as_ref()[..m]);
    })
});
