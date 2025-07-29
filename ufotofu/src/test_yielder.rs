use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{boxed::Box, vec::Vec};
#[cfg(feature = "std")]
use std::{boxed::Box, vec::Vec};

use arbitrary::Arbitrary;

// Heavily inspired by https://docs.rs/async-std/latest/src/async_std/task/yield_now.rs.html
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct TestYielder {
    pattern: Box<[bool]>, // nonzero length, at least one bool must be `false`
    index: usize,
}

impl TestYielder {
    pub fn new(pattern: Box<[bool]>) -> TestYielder {
        if pattern.iter().all(|b| *b) {
            // This also handles empty patterns.
            let mut pat = Vec::with_capacity(pattern.len() + 1);
            pat.extend_from_slice(&pattern);
            pat.push(false);

            TestYielder {
                pattern: pat.into_boxed_slice(),
                index: 0,
            }
        } else {
            TestYielder { pattern, index: 0 }
        }
    }

    #[inline]
    pub async fn maybe_yield(&mut self) {
        MaybeYield(self).await
    }
}

impl<'a> Arbitrary<'a> for TestYielder {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let pattern = Box::<[bool]>::arbitrary(u)?;
        Ok(Self::new(pattern))
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        Box::<[bool]>::size_hint(depth)
    }
}

struct MaybeYield<'s>(&'s mut TestYielder);

impl Future for MaybeYield<'_> {
    type Output = ();

    // The futures executor is implemented as a FIFO queue, so all this future
    // does is re-schedule the future back to the end of the queue, giving room
    // for other futures to progress.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let do_yield = self.0.pattern[self.0.index];
        self.0.index = (self.0.index + 1) % self.0.pattern.len();

        if do_yield {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
