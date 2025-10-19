#![no_main]

use futures::join;
use libfuzzer_sys::fuzz_target;

use either::Either::*;

use smol::net::{TcpListener, TcpStream};
use ufotofu::consumer::compat::writer::writer_to_bulk_consumer;
use ufotofu::prelude::*;
use ufotofu::producer::compat::reader::reader_to_bulk_producer;
use ufotofu::queues::new_fixed;

fuzz_target!(|data: (
    Box<[u8]>,
    usize,
    usize,
    usize,
    Vec<BulkConsumerOperation>,
    usize,
    Vec<BulkProducerOperation>
)| {
    let (
        input,
        sender_queue_capacity,
        rec_queue_capacity,
        con_scrambler_buffer_size,
        con_ops,
        pro_scrambler_buffer_size,
        pro_ops,
    ) = data;
    let sender_queue_capacity = sender_queue_capacity.clamp(1, 4096);
    let rec_queue_capacity = rec_queue_capacity.clamp(1, 4096);
    let con_scrambler_buffer_size = con_scrambler_buffer_size.clamp(1, 4096);
    let pro_scrambler_buffer_size = pro_scrambler_buffer_size.clamp(1, 4096);

    pollster::block_on(async {
        let send = async {
            let stream = TcpStream::connect("127.0.0.1:8087").await.unwrap();

            let mut sender = writer_to_bulk_consumer(stream, new_fixed(sender_queue_capacity))
                .bulk_scrambled(new_fixed(con_scrambler_buffer_size), con_ops);

            for datum in input.iter() {
                assert_eq!((), sender.consume(*datum).await.unwrap());
            }
            assert_eq!((), sender.close(()).await.unwrap());
        };

        let receive = async {
            let listener = TcpListener::bind("127.0.0.1:8087").await.unwrap();
            let (stream, _addr) = listener.accept().await.unwrap();

            let mut receiver = reader_to_bulk_producer(stream, new_fixed(rec_queue_capacity))
                .bulk_scrambled(new_fixed(pro_scrambler_buffer_size), pro_ops);

            for datum in input.iter() {
                assert_eq!(Left(*datum), receiver.produce().await.unwrap());
            }

            assert_eq!(Right(()), receiver.produce().await.unwrap());
        };

        join!(receive, send);
    });
});
