# UFOTOFU

Abstractions for asynchronously working with series of data (“streams” and
“sinks”).

This crate provides alternatives to some abstractions of the popular
[`futures`](https://docs.rs/futures/latest/futures) crate:

- [`Producer`](https://docs.rs/ufotofu/latest/ufotofu/trait.Producer.html) and
  [`Consumer`](https://docs.rs/ufotofu/latest/ufotofu/trait.Consumer.html)
  replace
  [`Stream`](https://docs.rs/futures/latest/futures/prelude/trait.Stream.html)
  and [`Sink`](https://docs.rs/futures/latest/futures/prelude/trait.Sink.html),
  and
- [`BulkProducer`](https://docs.rs/ufotofu/latest/ufotofu/trait.BulkProducer.html)
  and
  [`BulkConsumer`](https://docs.rs/ufotofu/latest/ufotofu/trait.BulkConsumer.html)
  replace
  [`AsyncRead`](https://docs.rs/futures/latest/futures/prelude/trait.AsyncRead.html)
  [`AsyncWrite`](https://docs.rs/futures/latest/futures/prelude/trait.AsyncWrite.html).

## Fundamental Design Choices

- Async trait methods, no poll-based interfaces.
- `nostd` by default.
- Fatal errors, no resumption of processing after an error was signalled.
- Full generics for bulk operations, no restriction to `u8` and `io::Error`.
- Bulk processing generalises item-by-item processing; the bulk traits extend
  the item-by-item traits.
- Zero-copy bulk processing; the bulk traits _expose_ slices instead of copying
  into or from passed slices.
- Buffering is abstracted-over in traits, not provided by concrete structs.
- Emphasis on producer-consumer duality, neither is more expressive than the
  other.
- Producers emit a dedicated final value, consumers receive a dedicated value
  when closed.
- British spelling.

Read the [docs](https://docs.rs/ufotofu/latest/ufotofu/) for a thorough
introduction to the API, see the
[ufotofu website](https://ufotofu.worm-blossom.org/) for a discussion of the
design choices.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option. Unless you explicitly state
otherwise, any contribution intentionally submitted for inclusion in this crate
by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions.

---

This project was initially funded through the [NGI0 Core](https://nlnet.nl/core)
Fund, a fund established by [NLnet](https://nlnet.nl/) with financial support
from the European Commission's [Next Generation Internet](https://ngi.eu/)
programme, under the aegis of
[DG Communications Networks, Content and Technology](https://commission.europa.eu/about-european-commission/departments-and-executive-agencies/communications-networks-content-and-technology_en)
under grant agreement No
[101092990](https://cordis.europa.eu/project/id/101092990).
