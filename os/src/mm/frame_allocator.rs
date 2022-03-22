use super::{PhysAddr, PhysPageNum};
use crate::{config::MEMORY_END, sync::UPSafeCell};
use alloc::{collections::BinaryHeap, vec::Vec};
use core::fmt::{self, Debug, Formatter};
use lazy_static::*;

/// 物理页帧（RAII）
pub struct FrameTracker(PhysPageNum);

impl FrameTracker {
    /// 获取 `FrameTracker` 管理的物理页帧
    pub fn ppn(&self) -> PhysPageNum {
        self.0
    }
}

impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x?}", self.0))
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.0);
    }
}

trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

#[derive(Default)]
pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: BinaryHeap<usize>,
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self::default()
    }

    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn.into())
        } else if self.current == self.end {
            None
        } else {
            self.current += 1;
            Some((self.current - 1).into())
        }
    }

    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn: usize = ppn.into();
        if ppn == self.current - 1 {
            self.current = ppn;
            while let Some(ppn) = self.recycled.peek() {
                if *ppn == self.current - 1 {
                    self.current = self.recycled.pop().unwrap();
                } else {
                    break;
                }
            }
        } else if ppn < self.current {
            self.recycled.push(ppn);
        } else {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
    }
}

lazy_static! {
    pub static ref FRAME_ALLOCATOR: UPSafeCell<StackFrameAllocator> =
        unsafe { UPSafeCell::new(StackFrameAllocator::new()) };
}

/// 初始化物理页帧分配器
pub fn init_frame_allocator() {
    extern "C" {
        fn ekernel();
    }
    let mut allocator = FRAME_ALLOCATOR.exclusive_access();
    allocator.current = PhysAddr::from(ekernel as usize).page().into();
    allocator.end = PhysAddr::from(MEMORY_END).page().into();
    allocator.end += 1;
}

/// 申请物理页帧
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR.exclusive_access().alloc().map(|ppn| {
        ppn.get_bytes_array().fill(0);
        FrameTracker(ppn)
    })
}

/// 释放物理页帧
fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}

#[allow(unused)]
pub fn frame_allocator_test() {
    let mut v: Vec<FrameTracker> = Vec::new();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    v.clear();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    drop(v);
    println!("frame_allocator_test passed!");
}
