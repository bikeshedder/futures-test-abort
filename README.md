# futures-test-abort [![Latest Version](https://img.shields.io/crates/v/futures-test-abort.svg)](https://crates.io/crates/futures-test-abort) [![Build Status](https://travis-ci.org/bikeshedder/futures-test-abort.svg?branch=master)](https://travis-ci.org/bikeshedder/futures-test-abort)

This crate provides functions for testing the robustness of async libraries
when futures are aborted. A future is considered aborted when it is never
pulled to completion thus ending its execution prematurely.

Aborted futures are quite common when working with web servers like
[hyper](https://crates.io/crates/hyper) or
[actix-web](https://crates.io/crates/actix-web). When the client disconnects
the service handler function is not polled to completion.

### Example

The following code illustrates a quite common pattern when writing code. 

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_test_abort as fta;
use tokio::task::yield_now;

struct State {
    count: AtomicUsize
}

impl State {
    pub fn count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }
    pub fn increment_count(&self) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }
    pub fn decrement_count(&self) {
        self.count.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Default for State {
    fn default() -> Self {
         Self { count: AtomicUsize::default() }
    }
}

async fn do_something(state: &State) {
    // This is where things can go haywire. `state.count` is increased by one
    // and then another future is waited for. The code polling this future
    // might decide to stop polling this future at this point causing the
    // `state.count` to remain incremented by one.
    state.increment_count();
    do_more(state).await;
    state.decrement_count();
}

async fn do_more(state: &State) {
    assert!(state.count() > 0);
    yield_now().await;
}

#[tokio::main]
async fn main() {
    let mut state = State::default();
    // This is the normal case. The future gets polled to completion and
    // the count inside state is properly incremented and decremented.
    do_something(&mut state).await;
    assert_eq!(state.count(), 0);
    // This is the error case. The future gets polled only once and then
    // dropped. The count increments, then `do_more` yields and the future
    // is no longer polled. This causes the `count` to stay at `1`.
    fta::abort(do_something(&mut state), 1).await;
    assert_eq!(state.count(), 1);
}
```

### Example (fixed)

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_test_abort as fta;
use tokio::task::yield_now;

struct State {
    count: AtomicUsize
}

impl State {
    pub fn count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }
    pub fn increment_count(&self) -> IncrementGuard<'_> {
        // Instead of providing a `decrement_count` method an
        // `IncrementGuard` is returned which takes care of decrementing
        // the count when it is dropped. This makes it possilbe to clean
        // up the state even when the future is aborted.
        self.count.fetch_add(1, Ordering::Relaxed);
        IncrementGuard { state: self }
    }
}

impl Default for State {
    fn default() -> Self {
         Self { count: AtomicUsize::default() }
    }
}

#[must_use]
struct IncrementGuard<'a> {
    state: &'a State
}

impl<'a> Drop for IncrementGuard<'a> {
    fn drop(&mut self) {
        self.state.count.fetch_sub(1, Ordering::Relaxed);
    }
}

async fn do_something(state: &State) {
    let _guard = state.increment_count();
    do_more(state).await;
}

async fn do_more(state: &State) {
    assert!(state.count() > 0);
    yield_now().await;
}

#[tokio::main]
async fn main() {
    let state = State::default();
    // This is the normal case. Still works as expected.
    do_something(&state).await;
    assert_eq!(state.count(), 0);
    // Now the guard inside `do_something` ensures that the state is
    // rolled back no matter what. So aborting the async function is safe.
    fta::abort(do_something(&state), 1).await;
    assert_eq!(state.count(), 0);
}
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0)>
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT)>

at your option.
