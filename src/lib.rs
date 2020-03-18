use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct Aborted {
    num_polls: usize
}

struct AbortN<'a, T, F>
where F: Future<Output=T>
{
    num_polls: usize,
    max_polls: usize,
    future: Pin<&'a mut F>,
}

impl<'a, T, F> Future for AbortN<'a, T, F>
where F: Future<Output=T> {
    type Output=Result<T, Aborted>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.num_polls >= self.max_polls {
            return Poll::Ready(Err(Aborted {
                num_polls: self.num_polls
            }));
        }
        self.get_mut().num_polls += 1;
        match self.future.poll(cx) {
            Poll::Ready(v) => Poll::Ready(Ok(v)),
            Poll::Pending => Poll::Pending
        }
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

