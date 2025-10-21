use crate::fs::File;
use crate::mm::UserBuffer;
use crate::sync::mutex::{Mutex, MutexGuard};
use crate::sync::semaphore::Semaphore;
use crate::sync::up::UPSafeCell;
use crate::task::deadlock::GLOBAL_DEADLOCK_DETECTOR;
use alloc::sync::Arc;
use lazy_static::*;

// ID Allocators

lazy_static! {
    static ref MUTEX_ID_ALLOCATOR: UPSafeCell<usize> = unsafe { UPSafeCell::new(0) };
    static ref SEMAPHORE_ID_ALLOCATOR: UPSafeCell<usize> = unsafe { UPSafeCell::new(0) };
}

fn alloc_mutex_id() -> usize {
    let mut id = MUTEX_ID_ALLOCATOR.exclusive_access();
    *id += 1;
    *id
}

fn alloc_semaphore_id() -> usize {
    let mut id = SEMAPHORE_ID_ALLOCATOR.exclusive_access();
    *id += 1;
    *id
}

// MutexForUser

pub struct MutexForUser {
    id: usize,
    pub mutex: Arc<Mutex<()>>,
}

impl MutexForUser {
    pub fn new() -> Self {
        let id = alloc_mutex_id();
        GLOBAL_DEADLOCK_DETECTOR
            .exclusive_access()
            .mutex_detector
            .add_mutex(id);
        Self {
            id,
            mutex: Arc::new(Mutex::new(())),
        }
    }
    pub fn id(&self) -> usize {
        self.id
    }
}

impl Drop for MutexForUser {
    fn drop(&mut self) {
        GLOBAL_DEADLOCK_DETECTOR
            .exclusive_access()
            .mutex_detector
            .remove_mutex(self.id);
    }
}

impl File for MutexForUser {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        false
    }
    fn read(&self, _buf: UserBuffer) -> usize {
        panic!("Cannot read from a mutex!");
    }
    fn write(&self, _buf: UserBuffer) -> usize {
        panic!("Cannot write to a mutex!");
    }
}

// SemaphoreForUser

pub struct SemaphoreForUser {
    id: usize,
    pub semaphore: Arc<Semaphore>,
}

impl SemaphoreForUser {
    pub fn new(count: usize) -> Self {
        let id = alloc_semaphore_id();
        GLOBAL_DEADLOCK_DETECTOR
            .exclusive_access()
            .semaphore_detector
            .add_semaphore(id, count);
        Self {
            id,
            semaphore: Arc::new(Semaphore::new(count)),
        }
    }
    pub fn id(&self) -> usize {
        self.id
    }
}

impl Drop for SemaphoreForUser {
    fn drop(&mut self) {
        GLOBAL_DEADLOCK_DETECTOR
            .exclusive_access()
            .semaphore_detector
            .remove_semaphore(self.id);
    }
}

impl File for SemaphoreForUser {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        false
    }
    fn read(&self, _buf: UserBuffer) -> usize {
        panic!("Cannot read from a semaphore!");
    }
    fn write(&self, _buf: UserBuffer) -> usize {
        panic!("Cannot write to a semaphore!");
    }
}
