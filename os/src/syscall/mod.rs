//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.

use lazy_static::lazy_static;
use crate::sync::UPSafeCell;

/// write syscall
const SYSCALL_WRITE: usize = 64;
/// exit syscall
const SYSCALL_EXIT: usize = 93;
/// yield syscall
const SYSCALL_YIELD: usize = 124;
/// gettime syscall
const SYSCALL_GET_TIME: usize = 169;
/// trace syscall
const SYSCALL_TRACE: usize = 410;

const SYSCALL_ID_ARRAY: [usize; 5] = [
    SYSCALL_WRITE,
    SYSCALL_EXIT,
    SYSCALL_YIELD,
    SYSCALL_GET_TIME,
    SYSCALL_TRACE,
];

mod fs;
mod process;

use fs::*;
use process::*;

/// Audition authority for a SysCall
#[derive(Copy, Clone)]
pub struct SysCallAudit {
    id: usize,
    count: isize,
}

/// Array of Audition authority for all SysCalls.
pub struct SysCallAuditArray {
    array: [SysCallAudit; SYSCALL_ID_ARRAY.len()],
}

lazy_static! {
    /// Lazily loaded audit array
    pub static ref SYSCALL_AUDIT_ARRAY: UPSafeCell<SysCallAuditArray> = unsafe {
        UPSafeCell::new(SysCallAuditArray {
            array: {
                let mut arr = [SysCallAudit { id: 0, count: 0 }; SYSCALL_ID_ARRAY.len()];
                let mut i = 0;
                while i < SYSCALL_ID_ARRAY.len() {
                    arr[i] = SysCallAudit { id: SYSCALL_ID_ARRAY[i], count: 0 };
                    i += 1;
                }
                arr
            },
        })
    };
}

/// Reset syscall audit counts for a new task
pub fn reset_syscall_audit() {
    let array = &mut SYSCALL_AUDIT_ARRAY.exclusive_access().array;
    for i in 0..SYSCALL_ID_ARRAY.len() {
        array[i].count = 0;
    }
}

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    // Update audit count in a separate scope to release the borrow
    {
        let array = &mut SYSCALL_AUDIT_ARRAY.exclusive_access().array;
        for i in 0..SYSCALL_ID_ARRAY.len() {
            if array[i].id == syscall_id {
                array[i].count += 1;
                break;
            }
        }
    }

    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_TRACE => sys_trace(args[0], args[1], args[2]),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}