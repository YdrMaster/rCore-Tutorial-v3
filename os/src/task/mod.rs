mod context;
mod priority_queue;
mod switch;
mod task;

use crate::config::MAX_APP_NUM;
use crate::loader::{get_num_app, init_app_cx};
use crate::sync::UPSafeCell;
use lazy_static::*;
use switch::{__init, __switch};
use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;

pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

struct TaskManagerInner {
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current_task: usize,
}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::ZERO,
            task_status: TaskStatus::Exited,
        }; MAX_APP_NUM];
        for i in 0..num_app {
            tasks[i].task_cx = TaskContext::goto_restore(init_app_cx(i));
            tasks[i].task_status = TaskStatus::Ready;
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    fn run_first_task(&self) -> ! {
        let next_task_cx_ptr = {
            let mut inner = self.inner.exclusive_access();
            let task0 = unsafe { inner.tasks.get_unchecked_mut(0) };
            task0.task_status = TaskStatus::Running;
            &task0.task_cx as _
        };
        unsafe { __init(next_task_cx_ptr) };
        unreachable!("run_first_task");
    }

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    fn run_next_task(&self) {
        let current_task_cx_ptr = {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            let context = unsafe { inner.tasks.get_unchecked_mut(current) };
            if context.task_status == TaskStatus::Exited {
                core::ptr::null_mut()
            } else {
                &mut inner.tasks[current].task_cx as *mut TaskContext
            }
        };

        if let Some(next) = self.find_next_task() {
            let next_task_cx_ptr = {
                let mut inner = self.inner.exclusive_access();
                inner.tasks[next].task_status = TaskStatus::Running;
                inner.current_task = next;
                &inner.tasks[next].task_cx as *const _
            };
            if current_task_cx_ptr.is_null() {
                unsafe { __init(next_task_cx_ptr) };
            } else {
                unsafe { __switch(next_task_cx_ptr, current_task_cx_ptr) };
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

pub fn suspend_current_and_run_next() {
    TASK_MANAGER.mark_current_suspended();
    TASK_MANAGER.run_next_task();
}

pub fn exit_current_and_run_next() {
    TASK_MANAGER.mark_current_exited();
    TASK_MANAGER.run_next_task();
}
