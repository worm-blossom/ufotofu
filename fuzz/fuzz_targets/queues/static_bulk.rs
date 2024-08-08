#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::arbitrary;
use libfuzzer_sys::fuzz_target;

use std::collections::VecDeque;

use ufotofu_queues::Queue;
use ufotofu_queues::Static;

#[derive(Debug, Arbitrary)]
enum Operation<T> {
    Enqueue(T),
    Dequeue,
    BulkEnqueue(Vec<T>),
    BulkDequeue(u8),
}

fuzz_target!(|data: Vec<Operation<u8>>| {
    let operations = data;

    let mut control = VecDeque::new();
    let mut test = Static::<u8, 42>::new();

    for operation in operations {
        match operation {
            Operation::Enqueue(item) => {
                let control_result = if control.len() >= 42 {
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
            Operation::BulkEnqueue(items) => {
                let amount = test.bulk_enqueue(&items);
                for (count, item) in items.iter().enumerate() {
                    if count >= amount {
                        break;
                    } else {
                        control.push_back(*item);
                    }
                }
            }
            Operation::BulkDequeue(n) => {
                let n = n as usize;
                if n > 0 {
                    let mut control_buffer = vec![];
                    let mut test_buffer = vec![0; n];

                    let test_amount = test.bulk_dequeue(&mut test_buffer);
                    for _ in 0..test_amount {
                        if let Some(item) = control.pop_front() {
                            control_buffer.push(item);
                        }
                    }

                    assert_eq!(&test_buffer[..test_amount], &control_buffer[..test_amount]);
                }
            }
        }
    }
});
