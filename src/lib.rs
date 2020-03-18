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
        let mut me = Pin::into_inner(self);
        if me.num_polls >= me.max_polls {
            return Poll::Ready(Err(Aborted {
                num_polls: me.num_polls
            }));
        }
        me.num_polls += 1;
        match me.future.poll(cx) {
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

