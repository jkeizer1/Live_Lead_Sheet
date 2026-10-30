//! Waits for one of wgpu's `async` calls to finish.
//!
//! wgpu asks for the adapter and the device through `async` functions, but on
//! a desktop they finish at once. A Rust `async` function returns a `Future`:
//! a value that is polled until it says it is ready. This is the smallest way
//! to poll one on the current thread, so no executor crate is needed.

use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};

/// Wakes the thread that is waiting, when the future has progress to report.
struct ThreadWaker {
    waiting_thread: Thread,
}

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.waiting_thread.unpark();
    }
}

/// Runs `future` to completion on the calling thread.
pub fn block_on<F: Future>(future: F) -> F::Output {
    // Pinning keeps the future at one address while it is polled, which
    // `async` code requires.
    let mut pinned_future = Box::pin(future);
    let waker: Waker = Waker::from(Arc::new(ThreadWaker {
        waiting_thread: thread::current(),
    }));
    let mut context: Context<'_> = Context::from_waker(&waker);
    loop {
        match pinned_future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            // `park` may return early; polling again is harmless.
            Poll::Pending => thread::park(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ready_future_returns_its_value() {
        assert_eq!(block_on(async { 40 + 2 }), 42);
    }

    #[test]
    fn a_future_that_is_pending_once_is_polled_again() {
        struct PendingOnce {
            polled_before: bool,
        }
        impl Future for PendingOnce {
            type Output = u32;
            fn poll(mut self: std::pin::Pin<&mut Self>, context: &mut Context<'_>) -> Poll<u32> {
                if self.polled_before {
                    Poll::Ready(7)
                } else {
                    self.polled_before = true;
                    context.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }
        assert_eq!(
            block_on(PendingOnce {
                polled_before: false
            }),
            7
        );
    }
}
