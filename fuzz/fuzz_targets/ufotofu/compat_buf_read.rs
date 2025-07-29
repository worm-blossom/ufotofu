#![no_main]

use futures::join;
use libfuzzer_sys::fuzz_target;

use either::Either::*;

use smol::io::BufReader;
use smol::net::{TcpListener, TcpStream};
use ufotofu::consumer::{self, BulkConsumerOperation, WriterToBulkConsumer};
use ufotofu::producer::{self, BufReaderToBulkProducer, BulkProducerOperation};
use ufotofu::{Consumer, Producer};

use ufotofu_queues::Fixed;

fuzz_target!(|data: (
    Box<[u8]>,
    usize,
    Vec<BulkConsumerOperation>,
    Vec<BulkProducerOperation>
)| {
    let (input, sender_queue_capacity, consume_ops, produce_ops) = data;

    let sender_queue_capacity = sender_queue_capacity.clamp(1, 2048);

    pollster::block_on(async {
        let send = async {
            let stream = TcpStream::connect("127.0.0.1:8087").await.unwrap();

            let sender_queue: Fixed<u8> = Fixed::new(sender_queue_capacity);
            let sender = WriterToBulkConsumer::new(stream, sender_queue);
            let mut sender = consumer::BulkScrambler::new(sender, consume_ops);

            for datum in input.iter() {
                assert_eq!((), sender.consume(*datum).await.unwrap());
            }
            assert_eq!((), sender.close(()).await.unwrap());
        };

        let receive = async {
            let listener = TcpListener::bind("127.0.0.1:8087").await.unwrap();
            let (stream, _addr) = listener.accept().await.unwrap();
            let stream = BufReader::new(stream);

            let receiver = BufReaderToBulkProducer::new(stream);
            let mut receiver = producer::BulkScrambler::new(receiver, produce_ops);

            for datum in input.iter() {
                assert_eq!(Left(*datum), receiver.produce().await.unwrap());
            }

            assert_eq!(Right(()), receiver.produce().await.unwrap());
        };

        join!(receive, send);
    });
});
