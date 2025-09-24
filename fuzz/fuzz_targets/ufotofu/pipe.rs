// #![no_main]

// use libfuzzer_sys::fuzz_target;
// use ufotofu::{consumer::TestConsumer, pipe, producer::TestProducer, PipeError};

// fuzz_target!(
//     |data: (TestProducer<u16, u16, u16>, TestConsumer<u16, u16, u16>)| {
//         pollster::block_on(async {
//             let (mut pro, mut con) = data;

//             let items = pro.remaining().to_vec();
//             let last = *pro.peek_last().unwrap();
//             let consumer_err = *con.peek_error().unwrap();

//             match pipe(&mut pro, &mut con).await {
//                 Ok(()) => {
//                     assert!(pro.did_already_emit_last());
//                     assert_eq!(con.consumed(), &items[..]);
//                     assert_eq!(con.final_consumed(), Some(&last.unwrap()));
//                 }
//                 Err(PipeError::Consumer(err)) => {
//                     if pro.peek_last().is_none() && last.is_err() {
//                         panic!();
//                     }
//                     assert!(con.did_error());
//                     assert_eq!(err, consumer_err);
//                     assert_eq!(con.consumed(), &items[..con.consumed().len()]);
//                     assert_eq!(
//                         pro.remaining(),
//                         &items[con.consumed().len()
//                             + if pro.did_already_emit_last() { 0 } else { 1 }..]
//                     );
//                 }
//                 Err(PipeError::Producer(err)) => {
//                     assert!(pro.did_already_emit_last());
//                     assert!(!con.did_error());
//                     assert_eq!(err, last.unwrap_err());
//                     assert_eq!(con.consumed(), &items[..con.consumed().len()]);
//                     assert_eq!(pro.remaining(), &items[con.consumed().len()..]);
//                 }
//             }
//         });
//     }
// );
