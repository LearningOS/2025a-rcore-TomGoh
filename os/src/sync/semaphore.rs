use crate::sync::UPSafeCell;
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use alloc::collections::VecDeque;
use alloc::sync::Arc;

/// A semaphore synchronization primitive
pub struct Semaphore {
    /// The inner state of the semaphore
    pub inner: UPSafeCell<SemaphoreInner>,
    max_value: usize,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    /// Create a new semaphore with the given resource count
    pub fn new(res_count: usize) -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                })
            },
            max_value: res_count,
        }
    }

    /// Increment the semaphore count (V operation)
    pub fn up(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                wakeup_task(task);
            }
        }
    }

    /// Decrement the semaphore count (P operation), blocking if necessary
    pub fn down(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();
        }
    }

    /// Get the current value of the semaphore
    pub fn current_value(&self) -> usize {
        let inner = self.inner.exclusive_access();
        if inner.count > 0 {
            inner.count as usize
        } else {
            0
        }
    }

    /// Get the maximum value of the semaphore
    pub fn max_value(&self) -> usize {
        self.max_value
    }
}