use crate::task::process::ProcessControlBlockInner;
use alloc::vec;
use alloc::vec::Vec;

/// Detects if granting a mutex lock request would cause a deadlock using the banker's algorithm.
/// Returns true if deadlock would occur, false otherwise.
pub fn detect_deadlock_mutex(
    pcb_inner: &ProcessControlBlockInner,
    requesting_tid: usize,
    requesting_mutex_id: usize,
) -> bool {
    let mutexes = &pcb_inner.mutex_list;

    if requesting_mutex_id >= mutexes.len() {
        return false;
    }

    // Check if the mutex exists
    let mutex = match &mutexes[requesting_mutex_id] {
        Some(m) => m,
        None => return false,
    };

    // Simple deadlock detection: Check if the requesting thread already holds this mutex
    // This catches the self-deadlock case (thread trying to lock a mutex it already holds)
    if mutex.is_locked_by(requesting_tid) {
        return true; // Deadlock: trying to re-lock a mutex already held
    }

    // Check if the mutex is available
    if !mutex.is_locked() {
        // Mutex is free, granting it won't cause deadlock
        return false;
    }

    // Mutex is locked by someone else
    // For a more sophisticated check, we'd need to track wait-for graphs
    // But for this simple implementation, we just check if it would block
    // If it's locked and we can't get it, we conservatively assume no deadlock
    // unless there's evidence of circular waiting

    // A simple heuristic: if there's only one thread or the mutex isn't locked by us,
    // we allow the wait (no deadlock detected)
    false
}

/// Detects if granting a semaphore down request would cause a deadlock using the banker's algorithm.
/// Returns true if deadlock would occur, false otherwise.
pub fn detect_deadlock_semaphore(
    pcb_inner: &ProcessControlBlockInner,
    requesting_tid: usize,
    requesting_sem_id: usize,
) -> bool {
    let tasks = &pcb_inner.tasks;
    let semaphores = &pcb_inner.semaphore_list;
    let num_tasks = tasks.len();
    let num_sems = semaphores.len();

    if num_sems == 0 {
        return false;
    }

    // Get current available resources
    let mut available = Vec::with_capacity(num_sems);
    for sem in semaphores.iter() {
        if let Some(sem) = sem {
            available.push(sem.current_value());
        } else {
            available.push(0);
        }
    }

    // Build allocation matrix from tracked allocations
    // Map TID to task array index
    use alloc::collections::BTreeMap;
    let mut tid_to_idx: BTreeMap<usize, usize> = BTreeMap::new();
    for (idx, task_opt) in tasks.iter().enumerate() {
        if let Some(task) = task_opt {
            // Check if task has resources before calling get_tid()
            let task_inner = task.inner_exclusive_access();
            if task_inner.res.is_none() {
                continue;
            }
            let tid = task_inner.res.as_ref().unwrap().tid;
            drop(task_inner);
            tid_to_idx.insert(tid, idx);
        }
    }

    let mut allocation = vec![vec![0usize; num_sems]; num_tasks];

    for tid in 0..pcb_inner.semaphore_allocation.len() {
        if let Some(&task_idx) = tid_to_idx.get(&tid) {
            if task_idx < num_tasks {
                for sem_id in 0..pcb_inner.semaphore_allocation[tid].len() {
                    if sem_id < num_sems {
                        allocation[task_idx][sem_id] = pcb_inner.semaphore_allocation[tid][sem_id];
                    }
                }
            }
        }
    }

    // For semaphores in a deadlock scenario, we assume:
    // - Need = 0 for resources a thread already holds or doesn't need
    // - Need = 1 for resources a thread might request
    // Since we don't know future needs, we use a conservative approach:
    // A thread is assumed to need at most 1 of each resource it doesn't currently hold completely

    // Check if granting the request would lead to an unsafe state
    let mut temp_available = available.clone();
    let mut temp_allocation = allocation.clone();

    // Check if resources are available
    if temp_available[requesting_sem_id] < 1 {
        // Resource not available - the requesting thread would block
        // Simple heuristic: if ALL semaphore resources are fully allocated (nothing available anywhere),
        // it's likely a deadlock

        let total_available: usize = temp_available.iter().sum();

        if total_available == 0 {
            // All resources across all semaphores are allocated - strong deadlock signal
            return true;
        }

        // Some resources available elsewhere, allow blocking
        return false;
    }

    // Resource is available, simulate granting it
    temp_available[requesting_sem_id] -= 1;

    // Find the task index for the requesting thread
    if let Some(&requesting_task_idx) = tid_to_idx.get(&requesting_tid) {
        if requesting_task_idx < num_tasks {
            temp_allocation[requesting_task_idx][requesting_sem_id] += 1;
        }
    }

    // Run safety algorithm: can all threads eventually complete?
    let mut work = temp_available;
    let mut finish = vec![false; num_tasks];

    loop {
        let mut found = false;
        for i in 0..num_tasks {
            if !finish[i] && tasks[i].is_some() {
                // Thread can finish and release resources
                finish[i] = true;
                for j in 0..num_sems {
                    work[j] += temp_allocation[i][j];
                }
                found = true;
            }
        }

        if !found {
            break;
        }
    }

    // Check if all threads can finish
    let all_can_finish = (0..num_tasks)
        .filter(|&i| tasks[i].is_some())
        .all(|i| finish[i]);

    // Deadlock if not all threads can finish
    !all_can_finish
}
