use crate::task::{exit_current_and_run_next, run_next_task, set_priority};
use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    unreachable!("sys_exit");
}

pub fn sys_yield() -> isize {
    run_next_task();
    0
}

pub fn sys_setpriority(prio: isize) -> isize {
    if prio >= 2 {
        set_priority(prio);
        prio
    } else {
        -1
    }
}

pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}
