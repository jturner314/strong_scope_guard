# `strong_scope_guard`

[![Build status](https://travis-ci.org/jturner314/strong_scope_guard.svg?branch=master)](https://travis-ci.org/jturner314/strong_scope_guard)
[![Dependencies status](https://deps.rs/repo/github/jturner314/strong_scope_guard/status.svg)](https://deps.rs/repo/github/jturner314/strong_scope_guard)
[![Crate](https://img.shields.io/crates/v/strong_scope_guard.svg)](https://crates.io/crates/strong_scope_guard)
[![Documentation](https://docs.rs/strong_scope_guard/badge.svg)](https://docs.rs/strong_scope_guard)

This crate provides scope guards that can be relied upon for memory safety.
This is a workaround for Rust's lack of a guarantee that destructors will be
called. **This crate is experimental and under development.**

This crate provides two key features:

1. There is a guard type that can be passed to functions for setting up the
   guard and can be stored in user-defined types.

2. The guard's closure is guaranteed to run unless the program exits or aborts.
   In particular, there is no way to leak a scope guard in user code (with the
   exception of guards protecting the `'static` lifetime), so the guard can be
   used for ensuring memory safety.

Related crates are [`scopeguard`](https://crates.io/crates/scopeguard),
[`bulwark`](https://crates.io/crates/bulwark), and
[`drop_guard`](https://crates.io/crates/drop_guard), but they all have the same
limitation: they allow user code to take ownership of the guard and leak it,
causing the guard's closure to never be executed. As a result, execution of the
deferred closure cannot be relied upon for memory safety. This is not an issue
in most Rust code. However, it becomes important in embedded applications where
a peripheral can run concurrently with the main thread, such as DMA transfers
(memory safety) or running an ADC (preventing hardware damage).

## Contributing

Please feel free to create issues and submit PRs.

## License

Copyright 2018 Jim Turner

Licensed under the [Apache License, Version 2.0](LICENSE-APACHE) or the [MIT
license](LICENSE-MIT), at your option. You may not use this project except in
compliance with those terms.
