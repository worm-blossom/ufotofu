#![no_main]
use std::collections::VecDeque;

use libfuzzer_sys::arbitrary;
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use ufotofu_queues::Queue;
use ufotofu_queues::Static;

#[derive(Debug, Arbitrary)]
enum Operation<T> {
    Enqueue(T),
    Dequeue,
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
        }
    }
});
