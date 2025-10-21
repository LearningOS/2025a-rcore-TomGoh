use crate::task::current_task;
use crate::task::TaskControlBlock;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next, wakeup_task};
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;

/// Trait for mutex implementations
pub trait Mutex: Send + Sync {
    /// Lock the mutex
    fn lock(&self);
    /// Unlock the mutex
    fn unlock(&self);
    /// Check if the mutex is currently locked
    fn is_locked(&self) -> bool;
    /// Check if the mutex is locked by a specific thread ID
    fn is_locked_by(&self, tid: usize) -> bool;
}

/// A spin-lock based mutex implementation
pub struct MutexSpin {
    locked: UPSafeCell<bool>,
    owner: UPSafeCell<Option<usize>>,
}

impl MutexSpin {
    /// Create a new spin mutex
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
            owner: unsafe { UPSafeCell::new(None) },
        }
    }
}

impl Mutex for MutexSpin {
    fn lock(&self) {
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                let mut owner = self.owner.exclusive_access();
                *owner = Some(current_task().unwrap().get_tid());
                return;
            }
        }
    }

    fn unlock(&self) {
        let mut locked = self.locked.exclusive_access();
        let mut owner = self.owner.exclusive_access();
        // if owner is not current task, panic
        assert_eq!(*owner, Some(current_task().unwrap().get_tid()));
        *locked = false;
        *owner = None;
    }

    fn is_locked(&self) -> bool {
        *self.locked.exclusive_access()
    }

    fn is_locked_by(&self, tid: usize) -> bool {
        *self.owner.exclusive_access() == Some(tid)
    }
}

/// A blocking mutex implementation that puts waiting tasks to sleep
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
    owner: Option<usize>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                    owner: None,
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    fn lock(&self) {
        let mut inner = self.inner.exclusive_access();
        if inner.locked {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();
        } else {
            inner.locked = true;
            inner.owner = Some(current_task().unwrap().get_tid());
        }
    }

    fn unlock(&self) {
        let mut inner = self.inner.exclusive_access();
        assert!(inner.locked);
        assert_eq!(inner.owner, Some(current_task().unwrap().get_tid()));
        if let Some(task) = inner.wait_queue.pop_front() {
            inner.owner = Some(task.get_tid());
            wakeup_task(task);
        } else {
            inner.locked = false;
            inner.owner = None;
        }
    }

    fn is_locked(&self) -> bool {
        self.inner.exclusive_access().locked
    }

    fn is_locked_by(&self, tid: usize) -> bool {
        self.inner.exclusive_access().owner == Some(tid)
    }
}