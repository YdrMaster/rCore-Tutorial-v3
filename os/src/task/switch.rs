core::arch::global_asm!(include_str!("switch.S"));

use super::TaskContext;

extern "C" {
    pub fn __switch(next_task_cx_ptr: *const TaskContext, current_task_cx_ptr: *mut TaskContext);
    pub fn __init(next_task_cx_ptr: *const TaskContext);
}
