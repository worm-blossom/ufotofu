#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{bulk_pipe, prelude::*, PipeError};

fuzz_target!(
    |data: (TestProducer<u16, u16, u16>, TestConsumer<u16, u16, u16>)| {
        pollster::block_on(async {
            let (mut pro, mut con) = data;

            let items = pro.as_slice().to_vec();
            let last = *pro.peek_last().unwrap();
            let consumer_err = *con.peek_error().unwrap();

            match bulk_pipe(&mut pro, &mut con).await {
                Ok(()) => {
                    assert!(pro.did_already_emit_last());
                    assert_eq!(con.as_slice(), &items[..]);
                    assert_eq!(con.peek_final(), Some(&last.unwrap()));
                }
                Err(PipeError::Consumer(err)) => {
                    if pro.peek_last().is_none() && last.is_err() {
                        panic!();
                    }
                    assert!(con.did_already_error());
                    assert_eq!(err, consumer_err);
                    assert_eq!(con.as_slice(), &items[..con.as_slice().len()]);
                    assert_eq!(pro.as_slice(), &items[con.as_slice().len()..]);
                }
                Err(PipeError::Producer(err)) => {
                    assert!(pro.did_already_emit_last());
                    assert!(!con.did_already_error());
                    assert_eq!(err, last.unwrap_err());
                    assert_eq!(con.as_slice(), &items[..con.as_slice().len()]);
                    assert_eq!(pro.as_slice(), &items[con.as_slice().len()..]);
                }
            }
        });
    }
);
