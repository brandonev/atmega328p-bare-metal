//! Fixed-capacity queues for communication between execution contexts.

use crate::interrupts::with_disabled;
use core::cell::UnsafeCell;

/// An interrupt-safe wrapper around a fixed-capacity event queue.
///
/// Access is serialized by temporarily disabling interrupts.
pub struct SharedEventQueue<T: Copy, const N: usize> {
    inner: UnsafeCell<EventQueue<T, N>>,
}

impl<T: Copy, const N: usize> Default for SharedEventQueue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

// All access to inner is serialized by disabling interrupts
unsafe impl<T: Copy + Send, const N: usize> Sync for SharedEventQueue<T, N> {}

impl<T: Copy, const N: usize> SharedEventQueue<T, N> {
    /// Creates an empty shared queue with capacity `N`.
    ///
    /// # Panics
    ///
    /// Panics if `N` is zero.
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(EventQueue::new()),
        }
    }

    /// Adds an item to the back of the queue.
    ///
    /// Returns the rejected item if the queue is full.
    pub fn push(&self, value: T) -> Result<(), T> {
        with_disabled(|| {
            let queue = unsafe { &mut *self.inner.get() };
            queue.push(value)
        })
    }

    /// Removes and returns the oldest item.
    ///
    /// Returns `None` if the queue is empty.
    pub fn pop(&self) -> Option<T> {
        with_disabled(|| {
            let queue = unsafe { &mut *self.inner.get() };
            queue.pop()
        })
    }

    /// Returns the number of items currently stored.
    pub fn len(&self) -> usize {
        with_disabled(|| {
            let queue = unsafe { &*self.inner.get() };
            queue.len()
        })
    }

    /// Returns the queue's fixed capacity.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Returns `true` if the queue contains no items.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the queue has reached its capacity.
    pub fn is_full(&self) -> bool {
        self.len() == N
    }
}

/// A fixed-capacity first-in-first-out queue.
///
/// Storage is allocated statically and the queue performs no heap allocation.
pub struct EventQueue<T: Copy, const N: usize> {
    buffer: [Option<T>; N],
    head: usize,
    tail: usize,
    length: usize,
}

impl<T: Copy, const N: usize> Default for EventQueue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy, const N: usize> EventQueue<T, N> {
    /// Creates an empty queue with capacity `N`.
    ///
    /// # Panics
    ///
    /// Panics if `N` is zero.
    pub const fn new() -> Self {
        assert!(N > 0);

        Self {
            buffer: [None; N],
            head: 0,
            tail: 0,
            length: 0,
        }
    }

    /// Adds an item to the back of the queue.
    ///
    /// Returns the rejected item if the queue is full.
    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.is_full() {
            return Err(value);
        }

        self.buffer[self.tail] = Some(value);
        self.tail = (self.tail + 1) % N;
        self.length += 1;

        Ok(())
    }

    /// Removes and returns the oldest item.
    ///
    /// Returns `None` if the queue is empty.
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let value = self.buffer[self.head].take();

        self.head = (self.head + 1) % N;
        self.length -= 1;

        value
    }

    /// Returns the number of items currently stored.
    pub fn len(&self) -> usize {
        self.length
    }

    /// Returns the queue's fixed capacity.
    pub fn capacity(&self) -> usize {
        N
    }

    /// Returns `true` if the queue contains no items.
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Returns `true` if the queue has reached its capacity.
    pub fn is_full(&self) -> bool {
        self.length == N
    }
}

#[cfg(test)]
mod tests {
    use super::EventQueue;

    #[test]
    fn new_queue_is_empty() {
        let queue = EventQueue::<u8, 4>::new();

        assert_eq!(queue.len(), 0);
        assert_eq!(queue.capacity(), 4);
        assert!(queue.is_empty());
        assert!(!queue.is_full());
    }

    #[test]
    fn preserves_fifo_order() {
        let mut queue = EventQueue::<u8, 4>::new();

        assert_eq!(queue.push(10), Ok(()));
        assert_eq!(queue.push(20), Ok(()));
        assert_eq!(queue.push(30), Ok(()));

        assert_eq!(queue.pop(), Some(10));
        assert_eq!(queue.pop(), Some(20));
        assert_eq!(queue.pop(), Some(30));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn rejects_new_item_when_full() {
        let mut queue = EventQueue::<u8, 2>::new();

        assert_eq!(queue.push(10), Ok(()));
        assert_eq!(queue.push(20), Ok(()));
        assert_eq!(queue.push(30), Err(30));

        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pop(), Some(10));
        assert_eq!(queue.pop(), Some(20));
    }

    #[test]
    fn wraps_around_the_array() {
        let mut queue = EventQueue::<u8, 3>::new();

        queue.push(10).unwrap();
        queue.push(20).unwrap();
        queue.push(30).unwrap();

        assert_eq!(queue.pop(), Some(10));
        assert_eq!(queue.pop(), Some(20));

        queue.push(40).unwrap();
        queue.push(50).unwrap();

        assert_eq!(queue.pop(), Some(30));
        assert_eq!(queue.pop(), Some(40));
        assert_eq!(queue.pop(), Some(50));
    }
}
