#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::arbitrary;
use libfuzzer_sys::fuzz_target;

use std::collections::VecDeque;

use ufotofu_queues::Fixed;
use ufotofu_queues::Queue;

#[derive(Debug, Arbitrary)]
enum Operation<T> {
    Enqueue(T),
    Dequeue,
    BulkEnqueue(Vec<T>),
    BulkDequeue(u8),
}

fuzz_target!(|data: (Vec<Operation<u8>>, usize)| {
    let operations = data.0;
    let capacity = data.1;

    // Restrict capacity to between 1 and 2048 bytes (inclusive).
    if capacity < 1 || capacity > 2048 {
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
                    control.push_back(item.clone());
                    None
                };
                let test_result = test.enqueue(item.clone());
                assert_eq!(test_result, control_result);
            }
            Operation::Dequeue => {
                let control_result = control.pop_front();
                let test_result = test.dequeue();
                assert_eq!(test_result, control_result);
            }
            Operation::BulkEnqueue(items) => {
                let amount = test.bulk_enqueue(&items);
                for (count, item) in items.iter().enumerate() {
                    if count >= amount {
                        break;
                    } else {
                        control.push_back(item.clone());
                    }
                }
            }
            Operation::BulkDequeue(n) => {
                let n = n as usize;
                if n > 0 {
                    let mut control_buffer = vec![];
                    let mut test_buffer = vec![];
                    test_buffer.resize(n, 0_u8);

                    let test_amount = test.bulk_dequeue(&mut test_buffer);
                    for _ in 0..test_amount {
                        if let Some(item) = control.pop_front() {
                            control_buffer.push(item.clone());
                        }
                    }

                    assert_eq!(&test_buffer[..test_amount], &control_buffer[..test_amount]);
                }
            }
        }
    }
});
