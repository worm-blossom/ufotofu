extern crate alloc;

use core::fmt;

use crate::Queue;

/// A queue holding up to a certain number of items. The capacity is statically determined by a const parameter. Performs no allocations.
///
/// Use the methods of the [Queue] trait implementation to interact with the contents of the queue.
pub struct Static<T, const N: usize> {
    /// Buffer of memory, used as a ring-buffer.
    data: [T; N],
    /// Read index.
    read: usize,
    /// Amount of valid data.
    amount: usize,
}

impl<T: Default + Copy, const N: usize> Default for Static<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Default + Copy, const N: usize> Static<T, N> {
    /// Create a fixed-capacity queue.
    pub fn new() -> Self {
        Static {
            data: [T::default(); N],
            read: 0,
            amount: 0,
        }
    }
}

impl<T, const N: usize> Static<T, N> {
    fn is_data_contiguous(&self) -> bool {
        self.read + self.amount < N
    }

    /// Return a slice containing the next items that should be read.
    fn readable_slice(&mut self) -> &[T] {
        if self.is_data_contiguous() {
            &self.data[self.read..self.write_to()]
        } else {
            &self.data[self.read..]
        }
    }

    /// Return a slice containing the next slots that should be written to.
    fn writeable_slice(&mut self) -> &mut [T] {
        let capacity = N;
        let write_to = self.write_to();
        if self.is_data_contiguous() {
            &mut self.data[write_to..capacity]
        } else {
            &mut self.data[write_to..self.read]
        }
    }

    fn write_to(&self) -> usize {
        (self.read + self.amount) % N
    }
}

impl<T: Copy, const N: usize> Queue for Static<T, N> {
    type Item = T;

    /// Return the number of items in the queue.
    fn len(&self) -> usize {
        self.amount
    }

    /// Attempt to enqueue the next item.
    ///
    /// Will return the item if the queue is full at the time of calling.
    fn enqueue(&mut self, item: T) -> Option<T> {
        if self.amount == N {
            Some(item)
        } else {
            self.data[self.write_to()] = item;
            self.amount += 1;

            None
        }
    }

    /// Expose a non-empty slice of memory for the client code to fill with items that should
    /// be enqueued.
    ///
    /// Will return `None` if the queue is full at the time of calling.
    fn expose_slots(&mut self) -> Option<&mut [T]> {
        if self.amount == N {
            None
        } else {
            Some(self.writeable_slice())
        }
    }

    /// Inform the queue that `amount` many items have been written to the first `amount`
    /// indices of the `expose_slots` it has most recently exposed.
    ///
    /// #### Invariants
    ///
    /// Callers must have written into (at least) the `amount` many first `expose_slots` that
    /// were most recently exposed. Failure to uphold this invariant may cause undefined behavior.
    ///
    /// #### Safety
    ///
    /// The queue will assume the first `amount` many `expose_slots` that were most recently
    /// exposed to contain initialized memory after this call, even if the memory it exposed was
    /// originally uninitialized. Violating the invariants will cause the queue to read undefined
    /// memory, which triggers undefined behavior.
    fn consider_enqueued(&mut self, amount: usize) {
        self.amount += amount;
    }

    /// Attempt to dequeue the next item.
    ///
    /// Will return `None` if the queue is empty at the time of calling.
    fn dequeue(&mut self) -> Option<T> {
        if self.amount == 0 {
            None
        } else {
            let previous_read = self.read;
            // Advance the read index by 1 or reset to 0 if at capacity.
            self.read = (self.read + 1) % N;
            self.amount -= 1;

            Some(self.data[previous_read])
        }
    }

    /// Expose a non-empty slice of items to be dequeued.
    ///
    /// Will return `None` if the queue is empty at the time of calling.
    fn expose_items(&mut self) -> Option<&[T]> {
        if self.amount == 0 {
            None
        } else {
            Some(self.readable_slice())
        }
    }

    /// Mark `amount` many items as having been dequeued.
    ///
    /// #### Invariants
    ///
    /// Callers must not mark items as dequeued that had not previously been exposed by
    /// `expose_items`.
    fn consider_dequeued(&mut self, amount: usize) {
        self.read = (self.read + amount) % N;
        self.amount -= amount;
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for Static<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Static")
            .field("len", &self.amount)
            .field("data", &DataDebugger(self))
            .finish()
    }
}

pub struct DataDebugger<'q, T, const N: usize>(&'q Static<T, N>);

impl<'q, T: fmt::Debug, const N: usize> fmt::Debug for DataDebugger<'q, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();

        if self.0.is_data_contiguous() {
            for item in &self.0.data[self.0.read..self.0.write_to()] {
                list.entry(item);
            }
        } else {
            for item in &self.0.data[self.0.read..] {
                list.entry(item);
            }

            for item in &self.0.data[0..(self.0.amount - self.0.data[self.0.read..].len())] {
                list.entry(item);
            }
        }

        list.finish()
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;

    #[test]
    fn enqueues_and_dequeues_with_correct_amount() {
        let mut queue: Static<u8, 4> = Static::new();

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
        let mut queue: Static<u8, 4> = Static::new();
        let mut buf = [0; 4];

        let enqueue_amount = queue.bulk_enqueue(b"ufo");
        let dequeue_amount = queue.bulk_dequeue(&mut buf);

        assert_eq!(enqueue_amount, dequeue_amount);
    }

    #[test]
    fn returns_item_on_enqueue_when_queue_is_full() {
        let mut queue: Static<u8, 1> = Static::new();

        assert_eq!(queue.enqueue(7), None);

        assert_eq!(queue.enqueue(0), Some(0))
    }

    #[test]
    fn returns_none_on_dequeue_when_queue_is_empty() {
        let mut queue: Static<u8, 1> = Static::new();

        // Enqueue and then dequeue an item.
        let _ = queue.enqueue(7);
        let _ = queue.dequeue();

        // The queue is now empty.
        assert!(queue.dequeue().is_none());
    }

    #[test]
    fn returnes_none_on_enqueue_slots_when_none_are_available() {
        // Create a fixed queue that exposes four slots.
        let mut queue: Static<u8, 4> = Static::new();

        // Copy data to two of the available slots and call `consider_queued`.
        let data = b"tofu";
        let slots = queue.expose_slots().unwrap();
        slots[0..2].copy_from_slice(&data[0..2]);
        queue.consider_enqueued(2);

        // Copy data to two of the available slots and call `consider_queued`.
        let slots = queue.expose_slots().unwrap();
        slots[0..2].copy_from_slice(&data[0..2]);
        queue.consider_enqueued(2);

        // Make a third call to `expose_slots` after all available slots have been used.
        assert!(queue.expose_slots().is_none());
    }

    #[test]
    fn returns_none_on_dequeue_slots_when_none_are_available() {
        // Create a fixed queue that exposes four slots.
        let mut queue: Static<u8, 4> = Static::new();

        let data = b"tofu";
        let _amount = queue.bulk_enqueue(data);

        let _slots = queue.expose_items().unwrap();
        queue.consider_dequeued(4);

        // Make a second call to `expose_items` after all available slots have been used.
        assert!(queue.expose_items().is_none());
    }

    #[test]
    fn test_debug_impl() {
        let mut queue: Static<u8, 4> = Static::new();

        assert_eq!(queue.enqueue(7), None);
        assert_eq!(queue.enqueue(21), None);
        assert_eq!(queue.enqueue(196), None);
        assert_eq!(
            format!("{:?}", queue),
            "Static { len: 3, data: [7, 21, 196] }"
        );

        assert_eq!(queue.dequeue(), Some(7));
        assert_eq!(format!("{:?}", queue), "Static { len: 2, data: [21, 196] }");

        assert_eq!(queue.dequeue(), Some(21));
        assert_eq!(format!("{:?}", queue), "Static { len: 1, data: [196] }");

        assert_eq!(queue.enqueue(33), None);
        assert_eq!(format!("{:?}", queue), "Static { len: 2, data: [196, 33] }");

        assert_eq!(queue.enqueue(17), None);
        assert_eq!(
            format!("{:?}", queue),
            "Static { len: 3, data: [196, 33, 17] }"
        );

        assert_eq!(queue.enqueue(200), None);
        assert_eq!(
            format!("{:?}", queue),
            "Static { len: 4, data: [196, 33, 17, 200] }"
        );
    }
}
