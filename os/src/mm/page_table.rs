use super::{frame_alloc, FrameTracker, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use alloc::{format, vec, vec::Vec};
use bitflags::*;

bitflags! {
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

/// 页表项
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry(usize);

impl PageTableEntry {
    const EMPTY: Self = Self(0);

    fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        Self(usize::from(ppn) << 10 | flags.bits as usize)
    }

    pub fn ppn(&self) -> PhysPageNum {
        (self.0 >> 10 & ((1usize << 44) - 1)).into()
    }

    fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.0 as _).unwrap()
    }

    pub fn is_valid(&self) -> bool {
        self.flags().contains(PTEFlags::V)
    }

    pub fn readable(&self) -> bool {
        self.flags().contains(PTEFlags::R)
    }

    pub fn writable(&self) -> bool {
        self.flags().contains(PTEFlags::W)
    }

    pub fn executable(&self) -> bool {
        self.flags().contains(PTEFlags::X)
    }
}

/// 页表序号组，用于查页表
///
/// 页表序号组从虚地址产生，并保存当前查到哪一级了
#[derive(Clone, Copy)]
struct PageTabelIndices(usize);

impl From<VirtPageNum> for PageTabelIndices {
    fn from(vpn: VirtPageNum) -> Self {
        Self(usize::from(vpn) + 3 * Self::UNIT)
    }
}

impl PageTabelIndices {
    /// 一级页表序号的长度
    const ITEM_LEN: usize = 9;

    /// 一级页表序号的遮罩
    const ITEM_MASK: usize = (1 << Self::ITEM_LEN) - 1;

    /// 三级页表序号的总长度
    const CONTENT_LEN: usize = 3 * Self::ITEM_LEN;

    /// 记录页表级别的单位
    const UNIT: usize = 1 << Self::CONTENT_LEN;

    /// 剩余的页表级别 3 -> 2 -> 1 -> 0
    fn index(&self) -> usize {
        self.0 >> Self::CONTENT_LEN
    }

    /// 充回一级页表序号
    fn restore(&mut self) {
        self.0 += Self::UNIT;
    }

    /// 取出一级页表序号
    fn pop(&mut self) -> usize {
        self.0 -= Self::UNIT;
        (self.0 >> (self.index() * Self::ITEM_LEN)) & Self::ITEM_MASK
    }
}

pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn(),
            frames: vec![frame],
        }
    }

    /// Temporarily used to get arguments from user space.
    ///
    /// satp: Supervisor Address Translation and Protection
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

    /// 尝试获取一个页表项的可变引用
    ///
    /// 如果不存在，返回仍需创建的页表序号
    fn try_find_pte(
        &self,
        vpn: VirtPageNum,
    ) -> Result<&'static mut PageTableEntry, (PhysPageNum, PageTabelIndices)> {
        let mut indices = PageTabelIndices::from(vpn);
        let mut ppn = self.root_ppn;
        loop {
            let i = indices.pop();
            let pte = &mut ppn.get_pte_array()[i];
            if indices.index() == 0 {
                return Ok(pte);
            } else if !pte.is_valid() {
                indices.restore();
                return Err((ppn, indices));
            } else {
                ppn = pte.ppn();
            }
        }
    }

    /// 查找一个页表项
    ///
    /// 如果不存在，返回 None
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        self.try_find_pte(vpn).ok()
    }

    /// 查找页表项
    ///
    /// 如果中间级页表不存在则补上
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        match self.try_find_pte(vpn) {
            // 页表项存在，直接返回
            Ok(pte) => Some(pte),
            // 页表项不存在，从 `ppn` 开始逐级创建
            Err((mut ppn, mut indices)) => loop {
                let i = indices.pop();
                let pte = &mut ppn.get_pte_array()[i];
                if indices.index() == 0 {
                    break Some(pte);
                } else {
                    let frame = frame_alloc().unwrap();
                    *pte = PageTableEntry::new(frame.ppn(), PTEFlags::V);
                    self.frames.push(frame);
                    ppn = pte.ppn();
                }
            },
        }
    }

    /// 建立 `vpn` -> `ppn` 的映射
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    /// 断开 `vpn` 的映射
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self
            .find_pte(vpn)
            .expect(&format!("vpn {:?} is invalid before unmapping", vpn));
        *pte = PageTableEntry::EMPTY;
    }

    /// 通过软件查找虚地址对应的页表项，用于内核访问用户内存
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).cloned()
    }

    /// 将根页表的物理页号按 satp 的格式标记为 Sv39
    pub fn token(&self) -> usize {
        const SV39: usize = 8;
        SV39 << 60 | usize::from(self.root_ppn)
    }
}

/// 将一个用 `ptr` 和 `len` 表示的用户空间内存块转换到内核空间
pub fn translated_byte_buffer(token: usize, buf: &[u8]) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = buf.as_ptr() as usize;
    let end = start + buf.len();
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}
