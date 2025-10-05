extern crate alloc;

use core::{fmt, marker::PhantomData, mem::swap};

use crate::queues::{BoundedQueue, Queue};

/// A queue which buffers data in a contiguous slice.
///
/// Use the methods of the [Queue] trait to interact with the contents of the queue.
///
/// Created by the [`new_contiguous`](crate::queues::new_contiguous) and [`new_contiguous`](crate::queues::new_contiguous_with) functions.
pub struct Contiguous<S, T> {
    /// Buffer of memory, used as a ring-buffer.
    buffer: S,
    /// Read index.
    read: usize,
    /// Amount of valid data.
    amount: usize,
    /// The function we use to initialise buffer slots, or to reset them after having produced an item.
    initialise_memory: fn() -> T,
    _phantom: PhantomData<T>,
}

impl<S, T> Contiguous<S, T> {
    /// Creates a new queue, using the given value of type `S` as a buffer for items of type `T`.
    ///
    /// The `initialise_memory` function is used internally to ensure that all queue slots contain valid memory at all times. The specific choice of `T` returned by that funciton does not affect the observable semantics of the queue at all.
    ///
    /// You probably want `S` to implement `AsRef<[T]>` and `AsMut<[T]>`, otherwise the returned [`Contiguous`] does not implement [`Queue`].
    pub(crate) fn new(buffer: S, initialise_memory: fn() -> T) -> Self {
        Self {
            buffer,
            read: 0,
            amount: 0,
            initialise_memory,
            _phantom: PhantomData,
        }
    }
}

impl<S, T> Contiguous<S, T>
where
    S: AsRef<[T]> + AsMut<[T]>,
{
    fn is_data_contiguous(&self) -> bool {
        self.read + self.amount < self.bounded_capacity()
    }

    /// Returns a slice containing the next items that should be read.
    fn readable_slice(&mut self) -> &[T] {
        if self.is_data_contiguous() {
            &self.buffer.as_ref()[self.read..self.write_to()]
        } else {
            &self.buffer.as_ref()[self.read..]
        }
    }

    /// Returns a slice containing the next slots that should be written to.
    fn writeable_slice(&mut self) -> &mut [T] {
        let capacity = self.bounded_capacity();
        let write_to = self.write_to();
        if self.is_data_contiguous() {
            &mut self.buffer.as_mut()[write_to..capacity]
        } else {
            &mut self.buffer.as_mut()[write_to..self.read]
        }
    }

    fn write_to(&self) -> usize {
        (self.read + self.amount) % self.bounded_capacity()
    }
}

impl<S, T> Queue for Contiguous<S, T>
where
    S: AsRef<[T]> + AsMut<[T]>,
{
    type Item = T;

    fn len(&self) -> usize {
        self.amount
    }

    fn is_full(&self) -> bool {
        self.amount == self.buffer.as_ref().len()
    }

    fn max_capacity(&self) -> Option<usize> {
        Some(self.bounded_capacity())
    }

    fn enqueue(&mut self, item: T) -> Option<T> {
        if self.amount == self.bounded_capacity() {
            Some(item)
        } else {
            let write_to = self.write_to();
            self.buffer.as_mut()[write_to] = item;
            self.amount += 1;

            None
        }
    }

    async fn expose_slots<F, R>(&mut self, f: F) -> R
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R),
    {
        let (amount, ret) = f(self.writeable_slice()).await;
        self.amount += amount;
        ret
    }

    fn dequeue(&mut self) -> Option<T> {
        if self.amount == 0 {
            None
        } else {
            let previous_read = self.read;
            // Advance the read index by 1 or reset to 0 if at capacity.
            self.read = (self.read + 1) % self.bounded_capacity();
            self.amount -= 1;

            let mut tmp = (self.initialise_memory)();
            swap(&mut self.buffer.as_mut()[previous_read], &mut tmp);

            Some(tmp)
        }
    }

    async fn expose_items<F, R>(&mut self, f: F) -> R
    where
        F: AsyncFnOnce(&[Self::Item]) -> (usize, R),
    {
        let (amount, ret) = f(self.readable_slice()).await;
        self.read = (self.read + amount) % self.bounded_capacity();
        self.amount -= amount;
        ret
    }
}

impl<S, T: fmt::Debug> fmt::Debug for Contiguous<S, T>
where
    S: AsRef<[T]> + AsMut<[T]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Contiguous")
            .field("len", &self.amount)
            .field("data", &DataDebugger(self))
            .finish()
    }
}

struct DataDebugger<'q, S, T>(&'q Contiguous<S, T>);

impl<S, T: fmt::Debug> fmt::Debug for DataDebugger<'_, S, T>
where
    S: AsRef<[T]> + AsMut<[T]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();

        if self.0.is_data_contiguous() {
            for item in &self.0.buffer.as_ref()[self.0.read..self.0.write_to()] {
                list.entry(item);
            }
        } else {
            for item in &self.0.buffer.as_ref()[self.0.read..] {
                list.entry(item);
            }

            for item in &self.0.buffer.as_ref()
                [0..(self.0.amount - self.0.buffer.as_ref()[self.0.read..].len())]
            {
                list.entry(item);
            }
        }

        list.finish()
    }
}

impl<S, T> BoundedQueue for Contiguous<S, T>
where
    S: AsRef<[T]> + AsMut<[T]>,
{
    fn bounded_capacity(&self) -> usize {
        self.buffer.as_ref().len()
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use crate::queues::QueueExt;

    use super::*;

    #[test]
    fn enqueues_and_dequeues_with_correct_amount() {
        let mut queue: Contiguous<[u8; 4], u8> = Contiguous::new([0; 4], Default::default);

        assert_eq!(queue.enqueue(7), None);
        assert_eq!(queue.enqueue(21), None);
        assert_eq!(queue.enqueue(196), None);
        assert_eq!(queue.len(), 3);

        assert_eq!(queue.enqueue(233), None);
        assert_eq!(queue.len(), 4);

        // Queue should be first-in, first-out.
        assert_eq!(queue.dequeue(), Some(7));
        assert_eq!(queue.len(), 3);
    }

    #[test]
    fn bulk_enqueues_and_dequeues_with_correct_amount() {
        pollster::block_on(async {
            let mut queue: Contiguous<[u8; 4], u8> = Contiguous::new([0; 4], Default::default);
            let mut buf = [0; 4];

            let enqueue_amount = queue.bulk_enqueue(b"ufo").await;
            let dequeue_amount = queue.bulk_dequeue(&mut buf).await;

            assert_eq!(enqueue_amount, dequeue_amount);
        });
    }

    #[test]
    fn returns_item_on_enqueue_when_queue_is_full() {
        let mut queue: Contiguous<[u8; 1], u8> = Contiguous::new([0], Default::default);

        assert_eq!(queue.enqueue(7), None);

        assert_eq!(queue.enqueue(0), Some(0))
    }

    #[test]
    fn returns_none_on_dequeue_when_queue_is_empty() {
        let mut queue: Contiguous<[u8; 1], u8> = Contiguous::new([0], Default::default);

        // Enqueue and then dequeue an item.
        let _ = queue.enqueue(7);
        let _ = queue.dequeue();

        // The queue is now empty.
        assert!(queue.dequeue().is_none());
    }

    #[test]
    fn test_debug_impl() {
        let mut queue: Contiguous<[u8; 4], u8> = Contiguous::new([0; 4], Default::default);

        assert_eq!(queue.enqueue(7), None);
        assert_eq!(queue.enqueue(21), None);
        assert_eq!(queue.enqueue(196), None);
        assert_eq!(
            format!("{:?}", queue),
            "Contiguous { len: 3, data: [7, 21, 196] }"
        );

        assert_eq!(queue.dequeue(), Some(7));
        assert_eq!(
            format!("{:?}", queue),
            "Contiguous { len: 2, data: [21, 196] }"
        );

        assert_eq!(queue.dequeue(), Some(21));
        assert_eq!(format!("{:?}", queue), "Contiguous { len: 1, data: [196] }");

        assert_eq!(queue.enqueue(33), None);
        assert_eq!(
            format!("{:?}", queue),
            "Contiguous { len: 2, data: [196, 33] }"
        );

        assert_eq!(queue.enqueue(17), None);
        assert_eq!(
            format!("{:?}", queue),
            "Contiguous { len: 3, data: [196, 33, 17] }"
        );

        assert_eq!(queue.enqueue(200), None);
        assert_eq!(
            format!("{:?}", queue),
            "Contiguous { len: 4, data: [196, 33, 17, 200] }"
        );
    }
}
