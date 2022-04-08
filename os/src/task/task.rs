use super::TaskContext;
use crate::config::{kernel_stack_position, TRAP_CONTEXT};
use crate::mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::trap::{trap_handler, TrapContext};

pub struct TaskControlBlock {
    pub running: bool,
    context: TaskContext,
    memory_set: MemorySet,
    trap_cx_ppn: PhysPageNum,
}

impl TaskControlBlock {
    pub fn context_ptr(&self) -> *const TaskContext {
        &self.context as _
    }

    pub fn context_ptr_mut(&mut self) -> *mut TaskContext {
        &mut self.context as _
    }

    pub fn trap_context_mut(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    pub fn user_token(&self) -> usize {
        self.memory_set.token()
    }

    pub fn mmap(&mut self, start: VirtAddr, end: VirtAddr, permission: MapPermission) -> isize {
        self.memory_set.mmap(start, end, permission)
    }

    pub fn munmap(&mut self, start: VirtAddr, end: VirtAddr) -> isize {
        self.memory_set.munmap(start, end)
    }

    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // map a kernel-stack in kernel space
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let task_control_block = Self {
            running: false,
            context: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
        };
        // prepare TrapContext in user space
        *task_control_block.trap_context_mut() = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }
}
