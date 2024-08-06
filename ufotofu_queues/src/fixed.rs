extern crate alloc;
use alloc::boxed::Box;

#[cfg(feature = "nightly")]
use alloc::alloc::{Allocator, Global};

#[cfg(not(feature = "nightly"))]
use alloc::vec::Vec;

use core::fmt;
use core::mem::MaybeUninit;

use crate::Queue;

#[cfg(not(feature = "nightly"))]
/// A queue holding up to a certain number of items. The capacity is set upon
/// creation and remains fixed. Performs a single heap allocation on creation.
///
/// Use the methods of the [Queue] trait implementation to interact with the contents of the queue.
pub struct Fixed<T> {
    /// Slice of memory, used as a ring-buffer.
    data: Box<[MaybeUninit<T>]>,
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
    data: Box<[MaybeUninit<T>], A>,
    /// Read index.
    read: usize,
    /// Amount of valid data.
    amount: usize,
}

impl<T> Fixed<T> {
    #[cfg(feature = "nightly")]
    /// Create a fixed-capacity queue. Panic if the initial memory allocation fails.
    pub fn new(capacity: usize) -> Self {
        Fixed {
            data: Box::new_uninit_slice(capacity),
            read: 0,
            amount: 0,
        }
    }

    #[cfg(feature = "nightly")]
    /// Try to create a fixed-capacity queue. If the initial memory allocation fails, return `None` instead.
    pub fn try_new(capacity: usize) -> Option<Self> {
        Some(Fixed {
            data: Box::try_new_uninit_slice(capacity).ok()?,
            read: 0,
            amount: 0,
        })
    }
}

#[cfg(not(feature = "nightly"))]
impl<T: Default> Fixed<T> {
    /// Create a fixed-capacity queue. Panic if the initial memory allocation fails.
    pub fn new(capacity: usize) -> Self {
        let mut v = Vec::with_capacity(capacity);
        v.resize_with(capacity, || MaybeUninit::uninit());

        Fixed {
            data: v.into_boxed_slice(),
            read: 0,
            amount: 0,
        }
    }
}

#[cfg(not(feature = "nightly"))]
impl<T> Fixed<T> {
    fn is_data_contiguous(&self) -> bool {
        self.read + self.amount < self.capacity()
    }

    /// Return a slice containing the next items that should be read.
    fn readable_slice(&mut self) -> &[MaybeUninit<T>] {
        if self.is_data_contiguous() {
            &self.data[self.read..self.write_to()]
        } else {
            &self.data[self.read..]
        }
    }

    /// Return a slice containing the next slots that should be written to.
    fn writeable_slice(&mut self) -> &mut [MaybeUninit<T>] {
        let capacity = self.capacity();
        let write_to = self.write_to();
        if self.is_data_contiguous() {
            &mut self.data[write_to..capacity]
        } else {
            &mut self.data[write_to..self.read]
        }
    }

    /// Return the capacity with which thise queue was initialised.
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
impl<T, A: Allocator> Fixed<T, A> {
    /// Create a fixed-capacity queue with a given memory allocator. Panic if the initial memory allocation fails.
    pub fn new_in(capacity: usize, alloc: A) -> Self {
        Fixed {
            data: Box::new_uninit_slice_in(capacity, alloc),
            read: 0,
            amount: 0,
        }
    }

    // /// Try to create a fixed-capacity queue with a given memory allocator. If the initial memory allocation fails, return `None` instead.
    // pub fn try_new_in(capacity: usize, alloc: A) -> Option<Self> {
    //     Some(Fixed {
    //         data: Box::try_new_uninit_slice_in(capacity, alloc)?,
    //         read: 0,
    //         amount: 0,
    //     })
    // }

    fn is_data_contiguous(&self) -> bool {
        self.read + self.amount < self.capacity()
    }

    /// Return a slice containing the next items that should be read.
    fn readable_slice(&mut self) -> &[MaybeUninit<T>] {
        if self.is_data_contiguous() {
            &self.data[self.read..self.write_to()]
        } else {
            &self.data[self.read..]
        }
    }

    /// Return a slice containing the next slots that should be written to.
    fn writeable_slice(&mut self) -> &mut [MaybeUninit<T>] {
        let capacity = self.capacity();
        let write_to = self.write_to();
        if self.is_data_contiguous() {
            &mut self.data[write_to..capacity]
        } else {
            &mut self.data[write_to..self.read]
        }
    }

    /// Return the capacity with which thise queue was initialised.
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
impl<T: Copy> Queue for Fixed<T> {
    type Item = T;

    /// Return the number of items in the queue.
    fn len(&self) -> usize {
        self.amount
    }

    /// Attempt to enqueue the next item.
    ///
    /// Will return the item if the queue is full at the time of calling.
    fn enqueue(&mut self, item: T) -> Option<T> {
        if self.amount == self.capacity() {
            Some(item)
        } else {
            self.data[self.write_to()].write(item);
            self.amount += 1;

            None
        }
    }

    /// Expose a non-empty slice of memory for the client code to fill with items that should
    /// be enqueued.
    ///
    /// Will return `None` if the queue is full at the time of calling.
    fn expose_slots(&mut self) -> Option<&mut [MaybeUninit<T>]> {
        if self.amount == self.capacity() {
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
    unsafe fn consider_enqueued(&mut self, amount: usize) {
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
            self.read = (self.read + 1) % self.capacity();
            self.amount -= 1;

            Some(unsafe { self.data[previous_read].assume_init() })
        }
    }

    /// Expose a non-empty slice of items to be dequeued.
    ///
    /// Will return `None` if the queue is empty at the time of calling.
    fn expose_items(&mut self) -> Option<&[T]> {
        if self.amount == 0 {
            None
        } else {
            Some(unsafe { crate::slice_assume_init_ref(self.readable_slice()) })
        }
    }

    /// Mark `amount` many items as having been dequeued.
    ///
    /// #### Invariants
    ///
    /// Callers must not mark items as dequeued that had not previously been exposed by
    /// `expose_items`.
    fn consider_dequeued(&mut self, amount: usize) {
        self.read = (self.read + amount) % self.capacity();
        self.amount -= amount;
    }
}

#[cfg(feature = "nightly")]
impl<T: Copy, A: Allocator> Queue for Fixed<T, A> {
    type Item = T;

    /// Return the number of items in the queue.
    fn len(&self) -> usize {
        self.amount
    }

    /// Attempt to enqueue the next item.
    ///
    /// Will return the item if the queue is full at the time of calling.
    fn enqueue(&mut self, item: T) -> Option<T> {
        if self.amount == self.capacity() {
            Some(item)
        } else {
            self.data[self.write_to()].write(item);
            self.amount += 1;

            None
        }
    }

    /// Expose a non-empty slice of memory for the client code to fill with items that should
    /// be enqueued.
    ///
    /// Will return `None` if the queue is full at the time of calling.
    fn expose_slots(&mut self) -> Option<&mut [MaybeUninit<T>]> {
        if self.amount == self.capacity() {
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
    unsafe fn consider_enqueued(&mut self, amount: usize) {
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
            self.read = (self.read + 1) % self.capacity();
            self.amount -= 1;

            Some(unsafe { self.data[previous_read].assume_init() })
        }
    }

    /// Expose a non-empty slice of items to be dequeued.
    ///
    /// Will return `None` if the queue is empty at the time of calling.
    fn expose_items(&mut self) -> Option<&[T]> {
        if self.amount == 0 {
            None
        } else {
            Some(unsafe { crate::slice_assume_init_ref(self.readable_slice()) })
        }
    }

    /// Mark `amount` many items as having been dequeued.
    ///
    /// #### Invariants
    ///
    /// Callers must not mark items as dequeued that had not previously been exposed by
    /// `expose_items`.
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
            .field("data", &DataDebugger(&self))
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
            for item in unsafe {
                crate::slice_assume_init_ref(&self.0.data[self.0.read..self.0.write_to()])
            } {
                list.entry(item);
            }
        } else {
            for item in unsafe { crate::slice_assume_init_ref(&self.0.data[self.0.read..]) } {
                list.entry(item);
            }

            for item in unsafe {
                crate::slice_assume_init_ref(
                    &self.0.data[0..(self.0.amount - self.0.data[self.0.read..].len())],
                )
            } {
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
            .field("data", &DataDebugger(&self))
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
            for item in unsafe {
                crate::slice_assume_init_ref(&self.0.data[self.0.read..self.0.write_to()])
            } {
                list.entry(item);
            }
        } else {
            for item in unsafe { crate::slice_assume_init_ref(&self.0.data[self.0.read..]) } {
                list.entry(item);
            }

            for item in unsafe {
                crate::slice_assume_init_ref(
                    &self.0.data[0..(self.0.amount - self.0.data[self.0.read..].len())],
                )
            } {
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
        let mut buf: [MaybeUninit<u8>; 4] = [MaybeUninit::uninit(); 4];

        let enqueue_amount = queue.bulk_enqueue(b"ufo");
        let dequeue_amount = queue.bulk_dequeue_uninit(&mut buf);

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
        crate::copy_from_slice(&mut slots[0..2], &data[0..2]);
        unsafe {
            queue.consider_enqueued(2);
        }

        // Copy data to two of the available slots and call `consider_queued`.
        let slots = queue.expose_slots().unwrap();
        crate::copy_from_slice(&mut slots[0..2], &data[0..2]);
        unsafe {
            queue.consider_enqueued(2);
        }

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
