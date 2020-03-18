# futures-test-abort [![Latest Version](https://img.shields.io/crates/v/futures-test-abort.svg)](https://crates.io/crates/futures-test-abort) [![Build Status](https://travis-ci.org/bikeshedder/futures-test-abort.svg?branch=master)](https://travis-ci.org/bikeshedder/futures-test-abort)

This crate contains functions for testing the robustness
of async libraries when futures are aborted. A future is
considered aborted when it is never pulled to completion
thus ending its execution.

Aborted futures are quite common when working with web
servers like [hyper](https://crates.io/crates/hyper) or
[actix-web](https://crates.io/crates/actix-web) as they
abort the service handler function when the client drops
the connection.

### Example

TODO

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0)>
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT)>

at your option.
