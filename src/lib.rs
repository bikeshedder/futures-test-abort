//! # futures-test-abort [![Latest Version](https://img.shields.io/crates/v/futures-test-abort.svg)](https://crates.io/crates/futures-test-abort) [![Build Status](https://travis-ci.org/bikeshedder/futures-test-abort.svg?branch=master)](https://travis-ci.org/bikeshedder/futures-test-abort)
//!
//! This crate provides functions for testing the robustness of async libraries
//! when futures are aborted. A future is considered aborted when it is never
//! pulled to completion thus ending its execution prematurely.
//!
//! Aborted futures are quite common when working with web servers like
//! [hyper](https://crates.io/crates/hyper) or
//! [actix-web](https://crates.io/crates/actix-web). When the client disconnects
//! the service handler function is not polled to completion.
//!
//! ### Example
//!
//! The following code illustrates a quite common pattern when writing code. 
//!
//! ```rust
//! use std::sync::atomic::{AtomicUsize, Ordering};
//!
//! use futures_test_abort as fta;
//! use tokio::task::yield_now;
//!
//! struct State {
//!     count: AtomicUsize
//! }
//!
//! impl State {
//!     pub fn count(&self) -> usize {
//!         self.count.load(Ordering::Relaxed)
//!     }
//!     pub fn increment_count(&self) {
//!         self.count.fetch_add(1, Ordering::Relaxed);
//!     }
//!     pub fn decrement_count(&self) {
//!         self.count.fetch_sub(1, Ordering::Relaxed);
//!     }
//! }
//!
//! impl Default for State {
//!     fn default() -> Self {
//!          Self { count: AtomicUsize::default() }
//!     }
//! }
//!
//! async fn do_something(state: &State) {
//!     // This is where things can go haywire. `state.count` is increased by one
//!     // and then another future is waited for. The code polling this future
//!     // might decide to stop polling this future at this point causing the
//!     // `state.count` to remain incremented by one.
//!     state.increment_count();
//!     do_more(state).await;
//!     state.decrement_count();
//! }
//!
//! async fn do_more(state: &State) {
//!     assert!(state.count() > 0);
//!     yield_now().await;
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut state = State::default();
//!     // This is the normal case. The future gets polled to completion and
//!     // the count inside state is properly incremented and decremented.
//!     do_something(&mut state).await;
//!     assert_eq!(state.count(), 0);
//!     // This is the error case. The future gets polled only once and then
//!     // dropped. The count increments, then `do_more` yields and the future
//!     // is no longer polled. This causes the `count` to stay at `1`.
//!     fta::abort(do_something(&mut state), 1).await;
//!     assert_eq!(state.count(), 1);
//! }
//! ```
//!
//! ### Example (fixed)
//!
//! ```rust
//! use std::sync::atomic::{AtomicUsize, Ordering};
//!
//! use futures_test_abort as fta;
//! use tokio::task::yield_now;
//!
//! struct State {
//!     count: AtomicUsize
//! }
//!
//! impl State {
//!     pub fn count(&self) -> usize {
//!         self.count.load(Ordering::Relaxed)
//!     }
//!     pub fn increment_count(&self) -> IncrementGuard<'_> {
//!         // Instead of providing a `decrement_count` method an
//!         // `IncrementGuard` is returned which takes care of decrementing
//!         // the count when it is dropped. This makes it possilbe to clean
//!         // up the state even when the future is aborted.
//!         self.count.fetch_add(1, Ordering::Relaxed);
//!         IncrementGuard { state: self }
//!     }
//! }
//!
//! impl Default for State {
//!     fn default() -> Self {
//!          Self { count: AtomicUsize::default() }
//!     }
//! }
//!
//! #[must_use]
//! struct IncrementGuard<'a> {
//!     state: &'a State
//! }
//!
//! impl<'a> Drop for IncrementGuard<'a> {
//!     fn drop(&mut self) {
//!         self.state.count.fetch_sub(1, Ordering::Relaxed);
//!     }
//! }
//!
//! async fn do_something(state: &State) {
//!     let _guard = state.increment_count();
//!     do_more(state).await;
//! }
//!
//! async fn do_more(state: &State) {
//!     assert!(state.count() > 0);
//!     yield_now().await;
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let state = State::default();
//!     // This is the normal case. Still works as expected.
//!     do_something(&state).await;
//!     assert_eq!(state.count(), 0);
//!     // Now the guard inside `do_something` ensures that the state is
//!     // rolled back no matter what. So aborting the async function is safe.
//!     fta::abort(do_something(&state), 1).await;
//!     assert_eq!(state.count(), 0);
//! }
//! ```
//!
//! ## License
//!
//! Licensed under either of
//!
//! - Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0)>
//! - MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT)>
//!
//! at your option.
#![warn(missing_docs)]

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// This error is returned when an `AbortN` future resolves
/// aborting the inner future.
#[derive(Debug)]
pub struct Aborted {
    /// Number of polls that were made before aborting the future.
    pub num_polls: usize
}

/// Wrapper for a `Future` which limits the times it can be polled.
pub struct Abort<T>
where
    T: Future
{
    num_polls: usize,
    max_polls: usize,
    future: T,
}

impl<T> Future for Abort<T>
where
    T: Future,
{
    type Output = Result<T::Output, Aborted>;
    
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.num_polls >= self.max_polls {
            return Poll::Ready(Err(Aborted {
                num_polls: self.num_polls
            }));
        }
        // Safety: we never move `self.num_polls` or `self.future`
        unsafe {
            let me = Pin::into_inner_unchecked(self);
            me.num_polls += 1;
            let future = Pin::new_unchecked(&mut me.future);
            match future.poll(cx) {
                Poll::Ready(v) => Poll::Ready(Ok(v)),
                Poll::Pending => Poll::Pending
            }
        }
    }
}

/// Create a `Abort` future wrapper which limits the times a future
/// can be polled before it returns `Err(Aborted(max_polls))`. If the
/// future is ready before reaching `max_polls` `Ok(T)` is returned
/// instead.
pub fn abort<T>(future: T, max_polls: usize) -> Abort<T>
where
    T: Future,
{
    Abort {
        num_polls: 0,
        max_polls,
        future,
    }
}

/// A future that never resolves but schedules itself to be continuously
/// polled.
pub struct Never;

impl Future for Never {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

/// Create a `Never` future which never resolves but schedules itself
/// to be woken all the time.
pub fn never() -> Never {
    Never
}

/// A future that is ready after a given number of polls.
pub struct After<T> {
    value: Option<T>,
    num_polls: usize,
    max_polls: usize,
}

impl<T> Future for After<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ready = self.num_polls >= self.max_polls;
        // Safety: we never move `self.num_polls`
        unsafe {
            let me = Pin::into_inner_unchecked(self);
            if ready {
                let value = me.value.take().unwrap();
                return Poll::Ready(value);
            }
            me.num_polls += 1;
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

/// Create future that is ready after a given number of polls.
pub fn after<T>(value: T, max_polls: usize) -> After<T> {
    After {
        value: Some(value),
        num_polls: 0,
        max_polls,
    }
}


#[cfg(test)]
mod tests {
    use crate::{abort, after, never};

    #[tokio::test]
    async fn abort_n_0_err() {
        assert!(abort(async { 42 }, 0).await.is_err());
    }

    #[tokio::test]
    async fn abort_n_1_ok() {
        let result = abort(async { 42usize }, 1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42usize);
    }

    #[tokio::test]
    async fn abort_n_err() {
        for max_polls in 0..100 {
            let result = abort(async { never().await }, max_polls).await;
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().num_polls, max_polls);
        }
    }

    #[tokio::test]
    async fn abort_n_ok() {
        for max_polls in 0..100 {
            let result = abort(async { after(max_polls, max_polls).await }, max_polls+1).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), max_polls);
        }
    }

}

