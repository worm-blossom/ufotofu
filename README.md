# UFOTOFU

Ufotofu provides APIs for lazily producing or consuming sequences of arbitrary length. Highlights of
ufotofu include

- consistent error handling semantics across all supported modes of sequence processing,
- meaningful subtyping relations between, for example, streams and readers,
- absence of needless specialization of error types or item types,
- fully analogous APIs for synchronous and asynchronous code,
- the ability to chain sequences of heterogenous types, and
- `nostd` support.

You can read an in-depth discussion of the API designs
[here](https://github.com/AljoschaMeyer/lazy_on_principle/blob/main/main.pdf).

## Core Abstractions

Ufotofu is built around a small hierarchy of traits that describe how to produce or consume a
sequence item by item.

A [`Producer`](sync::Producer) provides the items of a sequence to some client code, similar to the
[`futures::Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html) or the
[`core::iter::Iterator`] traits. Client code can repeatedly request the next item, and receives
either another item, an error, or a dedicated *final* item which may be of a different type than the
repeated items. An *iterator* of `T`s corresponds to a *producer* of `T`s with final item type `()`
and error type `!`.

A [`Consumer`](sync::Consumer) accepts the items of a sequence from some client code, similar to the
[`futures::Sink`](https://docs.rs/futures/latest/futures/sink/trait.Sink.html) traits. Client code
can repeatedly add new items to the sequence, until it adds a single *final* item which may be of a
different type than the repeated items.  A final item type of `()` makes adding the final item
equivalent to calling a conventional
[`close`](https://docs.rs/futures/latest/futures/sink/trait.Sink.html#tymethod.poll_close) method.

Producers and consumers are fully dual; the [pipe](sync::pipe) function writes as much data as
possible from a producer into a consumer.

Consumers often buffer items in an internal queue before performing side-effects on data in larger
chunks, such as writing data to the network only once a full packet can be filled. The
[`BufferedConsumer`](sync::BufferedConsumer) trait extends the [`Consumer`](sync::Consumer) trait to
allow client code to trigger effectful flushing of internal buffers. Dually, the
[`BufferedProducer`](sync::BufferedProducer) trait extends the [`Producer`](sync::Producer) trait to
allow client code to trigger effectful prefetching of data into internal buffers.

Finally, the [`BulkProducer`](sync::BulkProducer) and [`BulkConsumer`](sync::BulkConsumer) traits
extend [`BufferedProducer`](sync::BufferedProducer) and [`BufferedConsumer`](sync::BufferedConsumer)
respectively with the ability to operate on whole slices of items at a time, similar to
[`std::io::Read`] and [`std::io::Write`]. The [bulk_pipe](sync::bulk_pipe) function leverages this
ability to efficiently pipe data — unlike the standard library's [Read](std::io::Read) and
[Write](std::io::Write) traits, this is possible without allocating an auxilliary buffer.

## Crate Organisation

The ufotofu crate is split into three high-level modules:

- [`sync`] provides APIs for synchronous, blocking abstractions (think [`core::iter::Iterator`]),
- [`local_nb`] provides [`Future`](core::future::Future)-based, non-blocking APIs (think
  [`futures::stream::Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html)) for
*single-threaded* executors, and
- [`nb`] provides [`Future`](core::future::Future)-based, non-blocking APIs for *multi-threaded*
  executors.

All three modules implement the same concepts; the only differences are whether functions are
asynchronous, and, if so, whether futures implement [`Send`]. In particular, each module has its own
version of the core traits for interacting with sequences.

The [`nb`] module lacks most features of the [`sync`] and [`local_nb`] modules, but the core trait
definitions are there, and we happily accept pull-requests.

## Feature Flags

Ufotofu gates several features that are only interesting under certain circumstances behind feature
flags. These API docs document *all* functionality, though, as if all feature flags were activated.

All functionality that relies on the Rust standard library is gated behind the `std` feature flag
(enabled by default).

All functionality that performs dynamic memory allocations is gated behind the `alloc` feature flag
(disabled by default).

All functionality that aids in testing and development is gated behind the `dev` feature flag
(disabled by default).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or 
[MIT license](LICENSE-MIT) at your option.  Unless you explicitly state
otherwise, any contribution intentionally submitted for inclusion in this crate
by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions.
