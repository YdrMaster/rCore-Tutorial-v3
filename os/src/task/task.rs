use super::TaskContext;

#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    pub priority: usize,
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Exited,
}

pub const BIG_STRIDE: usize = 200 as _;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct TaskStride {
    pub stride: usize,
    pub index: usize,
}

impl PartialOrd for TaskStride {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TaskStride {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (self.stride.wrapping_sub(other.stride) as isize).cmp(&0)
    }
}
