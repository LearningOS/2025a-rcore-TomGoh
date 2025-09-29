//! Types related to task management

use super::TaskContext;
use crate::syscall::{SysCallAudit, SYSCALL_ID_ARRAY};

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// Syscall audit array for this task
    pub syscall_audit: [SysCallAudit; SYSCALL_ID_ARRAY.len()],
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
