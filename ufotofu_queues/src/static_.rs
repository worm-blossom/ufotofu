extern crate alloc;

use core::fmt;

use crate::Queue;

/// A queue holding up to a certain number of items. The capacity is statically determined by a const parameter. Performs no allocations.
///
/// Use the methods of the [Queue] trait to interact with the contents of the queue.
pub struct Static<T, const N: usize> {
    /// Buffer of memory, used as a ring-buffer.
    data: [T; N],
    /// Read index.
    read: usize,
    /// Amount of valid data.
    amount: usize,
}

impl<T: Default, const N: usize> Default for Static<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Default, const N: usize> Static<T, N> {
    /// Creates a statically-fixed-capacity queue.
    pub fn new() -> Self {
        Static {
            data: core::array::from_fn(|_| T::default()),
            read: 0,
            amount: 0,
        }
    }
}

impl<T, const N: usize> Static<T, N> {
    /// Creates a statically-fixed-capacity queue, using the given function to initially fill the queue (this is merely an internal operation to satisfy the type checker, these values are never actually dequeued).
    pub fn new_with_manual_tmps<TmpFun: FnMut() -> T>(mut create_tmp_value: TmpFun) -> Self {
        Static {
            data: core::array::from_fn(|_| create_tmp_value()),
            read: 0,
            amount: 0,
        }
    }
}

impl<T, const N: usize> Static<T, N> {
    fn is_data_contiguous(&self) -> bool {
        self.read + self.amount < N
    }

    /// Returns a slice containing the next items that should be read.
    fn readable_slice(&mut self) -> &[T] {
        if self.is_data_contiguous() {
            &self.data[self.read..self.write_to()]
        } else {
            &self.data[self.read..]
        }
    }

    /// Returns a slice containing the next slots that should be written to.
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

impl<T: Clone, const N: usize> Queue for Static<T, N> {
    type Item = T;

    fn len(&self) -> usize {
        self.amount
    }

    fn enqueue(&mut self, item: T) -> Option<T> {
        if self.amount == N {
            Some(item)
        } else {
            self.data[self.write_to()] = item;
            self.amount += 1;

            None
        }
    }

    fn expose_slots(&mut self) -> Option<&mut [T]> {
        if self.amount == N {
            None
        } else {
            Some(self.writeable_slice())
        }
    }

    fn consider_enqueued(&mut self, amount: usize) {
        self.amount += amount;
    }

    fn dequeue(&mut self) -> Option<T> {
        if self.amount == 0 {
            None
        } else {
            let previous_read = self.read;
            // Advance the read index by 1 or reset to 0 if at capacity.
            self.read = (self.read + 1) % N;
            self.amount -= 1;

            Some(self.data[previous_read].clone())
        }
    }

    fn expose_items(&mut self) -> Option<&[T]> {
        if self.amount == 0 {
            None
        } else {
            Some(self.readable_slice())
        }
    }

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

impl<T: fmt::Debug, const N: usize> fmt::Debug for DataDebugger<'_, T, N> {
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
