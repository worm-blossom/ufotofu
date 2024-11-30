extern crate alloc;
use alloc::boxed::Box;

#[cfg(feature = "nightly")]
use alloc::alloc::{Allocator, Global};

use alloc::vec::Vec;

use core::fmt;

use crate::Queue;

#[cfg(not(feature = "nightly"))]
/// A queue holding up to a certain number of items. The capacity is set upon
/// creation and remains fixed. Performs a single heap allocation on creation.
///
/// Use the methods of the [Queue] trait implementation to interact with the contents of the queue.
pub struct Fixed<T> {
    /// Slice of memory, used as a ring-buffer.
    data: Box<[T]>,
    /// Read index.
    read: usize,
    /// Amount of valid data.
    amount: usize,
}

#[cfg(feature = "nightly")]
/// A queue holding up to a certain number of items. The capacity is set upon
/// creation and remains fixed. Performs a single heap allocation on creation.
///
/// Use the methods of the [Queue] trait implementation to interact with the contents of the queue.
pub struct Fixed<T, A: Allocator = Global> {
    /// Slice of memory, used as a ring-buffer.
    data: Box<[T], A>,
    /// Read index.
    read: usize,
    /// Amount of valid data.
    amount: usize,
}

impl<T: Default> Fixed<T> {
    /// Creates a fixed-capacity queue. Panics if the initial memory allocation fails.
    pub fn new(capacity: usize) -> Self {
        let mut v = Vec::with_capacity(capacity);
        v.resize_with(capacity, Default::default);

        Fixed {
            data: v.into_boxed_slice(),
            read: 0,
            amount: 0,
        }
    }

    #[cfg(feature = "nightly")]
    /// Tries to create a fixed-capacity queue. If the initial memory allocation fails, returns `None` instead.
    pub fn try_new(capacity: usize) -> Option<Self> {
        let mut v = Vec::try_with_capacity(capacity).ok()?;
        v.resize_with(capacity, Default::default);

        Some(Fixed {
            data: v.into_boxed_slice(),
            read: 0,
            amount: 0,
        })
    }
}

#[cfg(not(feature = "nightly"))]
impl<T> Fixed<T> {
    fn is_data_contiguous(&self) -> bool {
        self.read + self.amount < self.capacity()
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
        let capacity = self.capacity();
        let write_to = self.write_to();
        if self.is_data_contiguous() {
            &mut self.data[write_to..capacity]
        } else {
            &mut self.data[write_to..self.read]
        }
    }

    /// Returns the capacity with which this queue was initialised.
    ///
    /// The number of free item slots at any time is `q.capacity() - q.amount()`.
    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    fn write_to(&self) -> usize {
        (self.read + self.amount) % self.capacity()
    }
}

#[cfg(feature = "nightly")]
impl<T: Default, A: Allocator> Fixed<T, A> {
    /// Creates a fixed-capacity queue with a given memory allocator. Panics if the initial memory allocation fails.
    pub fn new_in(capacity: usize, alloc: A) -> Self {
        let mut v = Vec::with_capacity_in(capacity, alloc);
        v.resize_with(capacity, Default::default);

        Fixed {
            data: v.into_boxed_slice(),
            read: 0,
            amount: 0,
        }
    }

    /// Tries to create a fixed-capacity queue. If the initial memory allocation fails, returns `None` instead.
    pub fn try_new_in(capacity: usize, alloc: A) -> Option<Self> {
        let mut v = Vec::try_with_capacity_in(capacity, alloc).ok()?;
        v.resize_with(capacity, Default::default);

        Some(Fixed {
            data: v.into_boxed_slice(),
            read: 0,
            amount: 0,
        })
    }
}

#[cfg(feature = "nightly")]
impl<T, A: Allocator> Fixed<T, A> {
    fn is_data_contiguous(&self) -> bool {
        self.read + self.amount < self.capacity()
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
        let capacity = self.capacity();
        let write_to = self.write_to();
        if self.is_data_contiguous() {
            &mut self.data[write_to..capacity]
        } else {
            &mut self.data[write_to..self.read]
        }
    }

    /// Returns the capacity with which this queue was initialised.
    ///
    /// The number of free item slots at any time is `q.capacity() - q.amount()`.
    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    fn write_to(&self) -> usize {
        (self.read + self.amount) % self.capacity()
    }
}

#[cfg(not(feature = "nightly"))]
impl<T: Clone> Queue for Fixed<T> {
    type Item = T;

    fn len(&self) -> usize {
        self.amount
    }

    fn enqueue(&mut self, item: T) -> Option<T> {
        if self.amount == self.capacity() {
            Some(item)
        } else {
            self.data[self.write_to()] = item;
            self.amount += 1;

            None
        }
    }

    fn expose_slots(&mut self) -> Option<&mut [T]> {
        if self.amount == self.capacity() {
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
            self.read = (self.read + 1) % self.capacity();
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
        self.read = (self.read + amount) % self.capacity();
        self.amount -= amount;
    }
}

#[cfg(feature = "nightly")]
impl<T: Clone, A: Allocator> Queue for Fixed<T, A> {
    type Item = T;

    fn len(&self) -> usize {
        self.amount
    }

    fn enqueue(&mut self, item: T) -> Option<T> {
        if self.amount == self.capacity() {
            Some(item)
        } else {
            self.data[self.write_to()] = item;
            self.amount += 1;

            None
        }
    }

    fn expose_slots(&mut self) -> Option<&mut [T]> {
        if self.amount == self.capacity() {
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
            self.read = (self.read + 1) % self.capacity();
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
        self.read = (self.read + amount) % self.capacity();
        self.amount -= amount;
    }
}

#[cfg(not(feature = "nightly"))]
impl<T: fmt::Debug> fmt::Debug for Fixed<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fixed")
            .field("capacity", &self.capacity())
            .field("len", &self.amount)
            .field("data", &DataDebugger(self))
            .finish()
    }
}

#[cfg(not(feature = "nightly"))]
pub struct DataDebugger<'q, T>(&'q Fixed<T>);

#[cfg(not(feature = "nightly"))]
impl<'q, T: fmt::Debug> fmt::Debug for DataDebugger<'q, T> {
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

#[cfg(feature = "nightly")]
impl<T: fmt::Debug, A: Allocator> fmt::Debug for Fixed<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fixed")
            .field("capacity", &self.capacity())
            .field("len", &self.amount)
            .field("data", &DataDebugger(self))
            .finish()
    }
}

#[cfg(feature = "nightly")]
pub struct DataDebugger<'q, T, A: Allocator = Global>(&'q Fixed<T, A>);

#[cfg(feature = "nightly")]
impl<'q, T: fmt::Debug, A: Allocator> fmt::Debug for DataDebugger<'q, T, A> {
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
        let mut queue: Fixed<u8> = Fixed::new(4);

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
        let mut queue: Fixed<u8> = Fixed::new(4);
        let mut buf = [0; 4];

        let enqueue_amount = queue.bulk_enqueue(b"ufo");
        let dequeue_amount = queue.bulk_dequeue(&mut buf);

        assert_eq!(enqueue_amount, dequeue_amount);
    }

    #[test]
    fn returns_item_on_enqueue_when_queue_is_full() {
        let mut queue: Fixed<u8> = Fixed::new(1);

        assert_eq!(queue.enqueue(7), None);

        assert_eq!(queue.enqueue(0), Some(0))
    }

    #[test]
    fn returns_none_on_dequeue_when_queue_is_empty() {
        let mut queue: Fixed<u8> = Fixed::new(1);

        // Enqueue and then dequeue an item.
        let _ = queue.enqueue(7);
        let _ = queue.dequeue();

        // The queue is now empty.
        assert!(queue.dequeue().is_none());
    }

    #[test]
    fn returnes_none_on_enqueue_slots_when_none_are_available() {
        // Create a fixed queue that exposes four slots.
        let mut queue: Fixed<u8> = Fixed::new(4);

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
        let mut queue: Fixed<u8> = Fixed::new(4);

        let data = b"tofu";
        let _amount = queue.bulk_enqueue(data);

        let _slots = queue.expose_items().unwrap();
        queue.consider_dequeued(4);

        // Make a second call to `expose_items` after all available slots have been used.
        assert!(queue.expose_items().is_none());
    }

    #[test]
    fn test_debug_impl() {
        let mut queue: Fixed<u8> = Fixed::new(4);

        assert_eq!(queue.enqueue(7), None);
        assert_eq!(queue.enqueue(21), None);
        assert_eq!(queue.enqueue(196), None);
        assert_eq!(
            format!("{:?}", queue),
            "Fixed { capacity: 4, len: 3, data: [7, 21, 196] }"
        );

        assert_eq!(queue.dequeue(), Some(7));
        assert_eq!(
            format!("{:?}", queue),
            "Fixed { capacity: 4, len: 2, data: [21, 196] }"
        );

        assert_eq!(queue.dequeue(), Some(21));
        assert_eq!(
            format!("{:?}", queue),
            "Fixed { capacity: 4, len: 1, data: [196] }"
        );

        assert_eq!(queue.enqueue(33), None);
        assert_eq!(
            format!("{:?}", queue),
            "Fixed { capacity: 4, len: 2, data: [196, 33] }"
        );

        assert_eq!(queue.enqueue(17), None);
        assert_eq!(
            format!("{:?}", queue),
            "Fixed { capacity: 4, len: 3, data: [196, 33, 17] }"
        );

        assert_eq!(queue.enqueue(200), None);
        assert_eq!(
            format!("{:?}", queue),
            "Fixed { capacity: 4, len: 4, data: [196, 33, 17, 200] }"
        );
    }
}
