#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{consumer::TestConsumer, pipe_at_most, producer::TestProducer, PipeError};

fuzz_target!(|data: (
    TestProducer<u16, u16, u16>,
    TestConsumer<u16, u16, u16>,
    usize
)| {
    pollster::block_on(async {
        let (mut pro, mut con, count) = data;

        let items = pro.remaining().to_vec();
        let last = *pro.peek_last().unwrap();
        let consumer_err = *con.peek_error().unwrap();

        match pipe_at_most(&mut pro, &mut con, count).await {
            Ok(amount) => {
                if amount < count {
                    assert!(pro.did_already_emit_last() || con.did_error());
                } else {
                    assert!(!pro.did_already_emit_last() && !con.did_error());
                }
                assert_eq!(con.consumed(), &items[..amount]);

                if amount < count {
                    assert_eq!(con.final_consumed(), Some(&last.unwrap()));
                }
            }
            Err(PipeError::Consumer(err)) => {
                if pro.peek_last().is_none() && last.is_err() {
                    panic!();
                }
                assert!(con.did_error());
                assert_eq!(err, consumer_err);
                assert_eq!(con.consumed(), &items[..con.consumed().len()]);
                assert_eq!(
                    pro.remaining(),
                    &items
                        [con.consumed().len() + if pro.did_already_emit_last() { 0 } else { 1 }..]
                );
                assert!(con.consumed().len() < count);
            }
            Err(PipeError::Producer(err)) => {
                assert!(pro.did_already_emit_last());
                assert!(!con.did_error());
                assert_eq!(err, last.unwrap_err());
                assert_eq!(con.consumed(), &items[..con.consumed().len()]);
                assert_eq!(pro.remaining(), &items[con.consumed().len()..]);
                assert!(con.consumed().len() < count);
            }
        }
    });
});
