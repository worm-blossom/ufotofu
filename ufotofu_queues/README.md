# UFOTOFU QUEUES

A [trait](Queue) and implementations of non-blocking, infallible queues that
support bulk enqueueing and bulk dequeueing via APIs inspired by
[ufotofu](https://crates.io/crates/ufotofu).

## Queue Implementations

So far, there are two implementations:

- [`Fixed`](https://docs.rs/ufotofu_queues/latest/ufotofu_queues/struct.Fixed.html),
which is a heap-allocated ring-buffer of unchanging capacity.
- [`Static`](https://docs.rs/ufotofu_queues/latest/ufotofu_queues/struct.Static.html), which works exactly like [`Fixed`](https://docs.rs/ufotofu_queues/latest/ufotofu_queues/struct.Fixed.html), but is backed by an array of static capacity. It requires no allocations.

Future plans include an elastic queue that grows and shrinks its capacity within certain parameters, to free up memory under low load.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or 
[MIT license](LICENSE-MIT) at your option.  Unless you explicitly state
otherwise, any contribution intentionally submitted for inclusion in this crate
by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions. 
