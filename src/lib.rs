use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug)]
pub struct Aborted {
    pub num_polls: usize
}

pub struct AbortN<T>
where
    T: Future
{
    num_polls: usize,
    max_polls: usize,
    future: T,
}

impl<T> Future for AbortN<T>
where
    T: Future,
{
    type Output = Result<T::Output, Aborted>;
    
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.num_polls >= self.max_polls {
            return Poll::Ready(Err(Aborted {
                num_polls: self.num_polls
            }));
        }
        // Safety: we never move `self.future`
        unsafe {
            let mut num_polls = self.as_mut().map_unchecked_mut(|me| &mut me.num_polls);
            *num_polls += 1;
            let future = self.as_mut().map_unchecked_mut(|me| &mut me.future);
            match future.poll(cx) {
                Poll::Ready(v) => Poll::Ready(Ok(v)),
                Poll::Pending => Poll::Pending
            }
        }
    }
}

pub fn abort_n<T>(future: T, max_polls: usize) -> AbortN<T>
where
    T: Future,
{
    AbortN {
        num_polls: 0,
        max_polls,
        future,
    }
}

pub struct Never;

impl Future for Never {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

pub fn never() -> Never {
    Never
}


#[cfg(test)]
mod tests {
    use crate::{abort_n, never};

    #[tokio::test]
    async fn abort_n_0_err() {
        assert!(abort_n(async { 42 }, 0).await.is_err());
    }

    #[tokio::test]
    async fn abort_n_1_ok() {
        let result = abort_n(async { 42usize }, 1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42usize);
    }

    #[tokio::test]
    async fn abort_n_err() {
        for max_polls in 0..100 {
            let result = abort_n(async { never().await }, max_polls).await;
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().num_polls, max_polls);
        }
    }

}

