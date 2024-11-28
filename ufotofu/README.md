# UFOTOFU

Ufotofu provides APIs for lazily producing or consuming sequences of arbitrary length. Highlights include

- bulk data transfer without temporary buffers,
- freely choosable error and item types, even for readers and writers,
- meaningful subtyping relations between streams and readers, and between sinks and writers,
- the ability to represent finite and infinite sequences on the type level, and
- `nostd` support.

Read the [docs here](https://docs.rs/ufotofu/latest/ufotofu/).

You can find an in-depth discussion of the API designs
[here](https://github.com/AljoschaMeyer/lazy_on_principle/blob/main/main.pdf).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or 
[MIT license](LICENSE-MIT) at your option.  Unless you explicitly state
otherwise, any contribution intentionally submitted for inclusion in this crate
by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions.

---

This project was initially funded through the [NGI0 Core](https://nlnet.nl/core) Fund, a fund established by [NLnet](https://nlnet.nl/) with financial support from the European Commission's [Next Generation Internet](https://ngi.eu/) programme, under the aegis of [DG Communications Networks, Content and Technology](https://commission.europa.eu/about-european-commission/departments-and-executive-agencies/communications-networks-content-and-technology_en) under grant agreement No [101092990](https://cordis.europa.eu/project/id/101092990).

