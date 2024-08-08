# UFOTOFU QUEUES

A [trait](Queue) and implementations of non-blocking, infallible queues that
support bulk enqueueing and bulk dequeueing via APIs inspired by
[ufotofu](https://crates.io/crates/ufotofu).

## Queue Implementations

So far, there is only a single implementation:
[`Fixed`](https://docs.rs/ufotofu_queues/0.1.0/ufotofu_queues/struct.Fixed.html),
which is a heap-allocated ring-buffer of unchanging capacity.

Future plans include a queue of static (known at compile-time) capacity that can
be used in allocator-less environments, and an elastic queue that grows and
shrinks its capacity within certain parameters, to free up memory under low
load.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or 
[MIT license](LICENSE-MIT) at your option.  Unless you explicitly state
otherwise, any contribution intentionally submitted for inclusion in this crate
by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions. 
