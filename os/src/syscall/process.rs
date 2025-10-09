//! Process management syscalls
use crate::{mm::{translated_byte_buffer, MapPermission, PTEFlags, PageTable, VirtAddr}, task::{change_program_brk, current_task_mmap, current_task_munmap, current_user_token, exit_current_and_run_next, get_task_syscall_count, suspend_current_and_run_next}, timer::get_time_ms};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let ms = get_time_ms();
    let sec = ms / 1000;
    let usec = (ms % 1000) * 1000;
    let time_val = TimeVal { sec, usec };

    let buffers = translated_byte_buffer(current_user_token(), ts as *mut u8, core::mem::size_of::<TimeVal>());
    let mut l = 0;
    unsafe {
        for buffer in buffers {
            let len = core::cmp::min(buffer.len(), core::mem::size_of::<TimeVal>() - l);
            buffer[..len].copy_from_slice(&core::slice::from_raw_parts((&time_val as *const TimeVal as *const u8).add(l), len));
            l += len;
        }
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 => {
            // Read one byte from address id and return its value
            let addr: VirtAddr = VirtAddr::from(id as usize);
            let perm = PTEFlags::U | PTEFlags::R;
            if PageTable::from_token(current_user_token()).check_permission(addr, perm) {
                let buffers = translated_byte_buffer(current_user_token(), id as *const u8, 1);
                buffers[0][0] as isize
            } else {
                -1
            }
        }
        1 => {
            // Write the lowest byte of data to address id
            let addr: VirtAddr = VirtAddr::from(id as usize);
            let perm = PTEFlags::U | PTEFlags::W;
            if PageTable::from_token(current_user_token()).check_permission(addr, perm) {
                let mut buffers = translated_byte_buffer(current_user_token(), id as *mut u8, 1);
                buffers[0][0] = (data & 0xff) as u8;
                0
            } else {
                -1
            }
        }
        2 => {
            // get the count by the syscall id for the current task
            get_task_syscall_count(id) as isize
        }
        _ => {
            return -1;
        }
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap");
    // check the alignment of start va
    let va = VirtAddr::from(start);
    if !va.aligned() {
        return -1;
    }
    let end_va = VirtAddr::from(start + len);
    // check the prot
    if prot & !0x7 != 0 || prot & 0x7 == 0 {
        return -1;
    }

    // manufacture the permission
    let readable = prot & 0x1 != 0;
    let writable = prot & 0x2 != 0;
    let executable = prot & 0x4 != 0;
    let mut map_perm = MapPermission::U;
    if readable {
        map_perm |= MapPermission::R;
    }
    if writable {
        map_perm |= MapPermission::W;
    }
    if executable {
        map_perm |= MapPermission::X;
    }

    // Use MmapManager to map the region
    if current_task_mmap(va, end_va, map_perm).is_ok() {
        0
    } else {
        -1
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");

    let va = VirtAddr::from(start);
    if !va.aligned() {
        return -1;
    }

    let end_va = VirtAddr::from(start + len);
    // Check if end address is also page-aligned
    if !end_va.aligned() {
        return -1;
    }

    // Use MmapManager to unmap the region
    if current_task_munmap(va, end_va).is_ok() {
        0
    } else {
        -1
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
