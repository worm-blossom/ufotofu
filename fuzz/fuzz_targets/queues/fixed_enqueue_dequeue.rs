#![no_main]
use std::collections::VecDeque;

use arbitrary::Arbitrary;
use libfuzzer_sys::arbitrary;
use libfuzzer_sys::fuzz_target;

use ufotofu_queues::Fixed;
use ufotofu_queues::Queue;

#[derive(Debug, Arbitrary)]
enum Operation<T> {
    Enqueue(T),
    Dequeue,
}

fuzz_target!(|data: (Vec<Operation<u8>>, usize)| {
    let operations = data.0;
    let capacity = data.1;

    // Restrict capacity to between 1 and 2048 bytes (inclusive).
    if !(1..=2048).contains(&capacity) {
        return;
    }

    let mut control = VecDeque::new();
    let mut test = Fixed::new(capacity);

    for operation in operations {
        match operation {
            Operation::Enqueue(item) => {
                let control_result = if control.len() >= capacity {
                    Some(item)
                } else {
                    control.push_back(item);
                    None
                };
                let test_result = test.enqueue(item);
                assert_eq!(test_result, control_result);
            }
            Operation::Dequeue => {
                let control_result = control.pop_front();
                let test_result = test.dequeue();
                assert_eq!(test_result, control_result);
            }
        }
    }
});
