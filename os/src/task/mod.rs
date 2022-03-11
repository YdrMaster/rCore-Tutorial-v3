mod context;
mod priority_queue;
mod switch;
mod task;

use crate::{
    config::MAX_APP_NUM,
    loader::{get_num_app, init_app_cx},
    sync::UPSafeCell,
};
use lazy_static::*;
use priority_queue::PriorityQueue;
use switch::{__init, __switch};
use task::{TaskControlBlock, TaskStatus, TaskStride};

pub use context::TaskContext;

use self::task::BIG_STRIDE;

pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

struct TaskManagerInner {
    queue: PriorityQueue<TaskStride, MAX_APP_NUM>,
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current: TaskStride,
}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut queue = PriorityQueue::default();
        let mut tasks = [TaskControlBlock {
            priority: 2,
            task_cx: TaskContext::ZERO,
            task_status: TaskStatus::Exited,
        }; MAX_APP_NUM];
        for i in 0..num_app {
            queue.push(TaskStride {
                stride: if i == 0 { 0 } else { 1 },
                index: i,
            });
            tasks[i].task_cx = TaskContext::goto_restore(init_app_cx(i));
            tasks[i].task_status = TaskStatus::Ready;
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    queue,
                    tasks,
                    current: TaskStride {
                        stride: 0,
                        index: 0,
                    },
                })
            },
        }
    };
}

impl TaskManager {
    fn run_next_task(&self, current_exited: bool) {
        let mut inner = self.inner.exclusive_access();
        // 更新当前任务状态，准备切换
        let TaskStride { stride, index } = inner.current;
        let context = unsafe { inner.tasks.get_unchecked_mut(index) };
        let current_task_cx_ptr = match context.task_status {
            TaskStatus::Ready => {
                // 当前任务就绪状态，仅出现在第一个任务
                // 实际此时没有任务在运行，因此不需要保存上下文
                core::ptr::null_mut()
            }
            TaskStatus::Running if current_exited => {
                // 当前任务已退出，不需要保存上下文
                context.task_status = TaskStatus::Exited;
                core::ptr::null_mut()
            }
            TaskStatus::Running => {
                // 当前任务耗尽时间片
                context.task_status = TaskStatus::Ready;
                let stride = stride.wrapping_add(BIG_STRIDE / context.priority);
                inner.queue.push(TaskStride { stride, index });
                &mut inner.tasks[index].task_cx as *mut TaskContext
            }
            TaskStatus::Exited => unreachable!(),
        };
        // 弹出下一个任务
        let next = inner.queue.pop().expect("All applications completed!");
        inner.current = next;
        // 更新当前任务状态
        let context = unsafe { inner.tasks.get_unchecked_mut(next.index) };
        let next_task_cx_ptr = &context.task_cx as *const _;
        context.task_status = TaskStatus::Running;
        // 释放 `inner` 准备切换控制流
        drop(inner);
        // 当前上下文不需要保存
        if current_task_cx_ptr.is_null() {
            unsafe { __init(next_task_cx_ptr) };
            unreachable!("Task exited!");
        } else {
            unsafe { __switch(next_task_cx_ptr, current_task_cx_ptr) };
            // go back to user mode
        }
    }

    fn set_priority(&self, value: isize) {
        let mut inner = self.inner.exclusive_access();
        let TaskStride { stride: _, index } = inner.current;
        inner.tasks[index].priority = value as _;
    }
}

pub fn set_priority(value: isize) {
    TASK_MANAGER.set_priority(value);
}

pub fn run_next_task() {
    TASK_MANAGER.run_next_task(false);
}

pub fn exit_current_and_run_next() {
    TASK_MANAGER.run_next_task(true);
}
