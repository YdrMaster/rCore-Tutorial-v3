use crate::{
    loader::{get_app_data, get_num_app},
    mm::MapPermission,
    sync::UPSafeCell,
    trap::TrapContext,
};
use alloc::collections::VecDeque;
use lazy_static::*;

mod context;
mod switch;
mod task;

use switch::{__init, __switch};
use task::TaskControlBlock;

pub use context::TaskContext;

pub struct TaskManager(UPSafeCell<VecDeque<TaskControlBlock>>);

lazy_static! {
    static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        TaskManager(unsafe {
            UPSafeCell::new(
                (0..num_app)
                    .map(|i| TaskControlBlock::new(get_app_data(i), i))
                    .collect(),
            )
        })
    };
}

impl TaskManager {
    fn get_current_token(&self) -> usize {
        self.0.exclusive_access().back().unwrap().user_token()
    }

    fn get_current_trap_cx(&self) -> &mut TrapContext {
        self.0.exclusive_access().back().unwrap().trap_context_mut()
    }

    fn mmap(&self, start: usize, len: usize, port: usize) -> isize {
        if start & (4096 - 1) != 0 || port & !0x7 != 0 || port == 0 {
            return -1;
        }

        let mut permission = MapPermission::U;
        if port & 0b001 != 0 {
            permission |= MapPermission::R;
        }
        if port & 0b010 != 0 {
            permission |= MapPermission::W;
        }
        if port & 0b100 != 0 {
            permission |= MapPermission::X;
        }

        self.0.exclusive_access().back_mut().unwrap().mmap(
            start.into(),
            (start + len).into(),
            permission,
        )
    }

    fn munmap(&self, start: usize, len: usize) -> isize {
        self.0
            .exclusive_access()
            .back_mut()
            .unwrap()
            .munmap(start.into(), (start + len).into())
    }

    fn run_next_task(&self, current_exited: bool) {
        let mut tasks = self.0.exclusive_access();
        let mut current = tasks.back_mut().unwrap();
        // 更新当前任务状态，准备切换
        let current_task_cx_ptr = if current.running {
            if current_exited {
                // 当前任务已退出，不需要保存上下文
                tasks.pop_back();
                core::ptr::null_mut()
            } else {
                // 当前任务耗尽时间片
                current.running = false;
                current.context_ptr_mut()
            }
        } else {
            // 当前任务就绪状态，仅出现在第一个任务
            // 实际此时没有任务在运行，因此不需要保存上下文
            core::ptr::null_mut()
        };
        // 弹出下一个任务
        let mut next = tasks.pop_front().expect("All applications completed!");
        let next_task_cx_ptr = next.context_ptr();
        next.running = true;
        tasks.push_back(next);
        // 释放 `inner` 准备切换控制流
        drop(tasks);
        // 当前上下文不需要保存
        if current_task_cx_ptr.is_null() {
            unsafe { __init(next_task_cx_ptr) };
            unreachable!("Task exited!");
        } else {
            unsafe { __switch(next_task_cx_ptr, current_task_cx_ptr) };
            // go back to user mode
        }
    }
}

pub fn run_next_task() {
    TASK_MANAGER.run_next_task(false);
}

pub fn exit_current_and_run_next() {
    TASK_MANAGER.run_next_task(true);
}

pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

pub fn mmap(start: usize, len: usize, port: usize) -> isize {
    TASK_MANAGER.mmap(start, len, port)
}

pub fn munmap(start: usize, len: usize) -> isize {
    TASK_MANAGER.munmap(start, len)
}
