use core::{fmt::Debug, mem::MaybeUninit};

/// 具有指定容量的优先队列（小顶堆）
pub struct PriorityQueue<T, const N: usize> {
    len: usize,
    val: [MaybeUninit<T>; N],
}

impl<T, const N: usize> Default for PriorityQueue<T, N> {
    fn default() -> Self {
        Self {
            len: 0,
            val: MaybeUninit::uninit_array(),
        }
    }
}

impl<T, const N: usize> PriorityQueue<T, N> {
    #[inline]
    fn get(&self, i: usize) -> &T {
        unsafe { self.val.get_unchecked(i).assume_init_ref() }
    }
}

impl<T: Ord, const N: usize> PriorityQueue<T, N> {
    pub fn push(&mut self, t: T) {
        if self.len == N {
            panic!("Out of capacity!");
        }

        self.val[self.len] = MaybeUninit::new(t);
        // 从底部上浮
        let mut i = self.len;
        while i > 0 {
            // 父节点的序号
            let j = i >> 1;
            // 交换或退出
            if self.get(i) < self.get(j) {
                self.val.swap(i, j);
                i = j;
            } else {
                break;
            }
        }

        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;
        self.val.swap(0, self.len);

        let n = self.len;
        // 从顶部下沉
        let mut i = 0;
        loop {
            // 较大子节点的序号
            let j = i * 2 + 1;
            let j = if n <= j {
                break;
            } else if n == j + 1 || self.get(j) < self.get(j + 1) {
                j
            } else {
                j + 1
            };
            // 交换或退出
            if self.get(i) > self.get(j) {
                self.val.swap(i, j);
                i = j;
            } else {
                break;
            }
        }
        unsafe {
            Some(
                core::mem::replace(
                    //
                    self.val.get_unchecked_mut(n),
                    MaybeUninit::uninit(),
                )
                .assume_init(),
            )
        }
    }
}

impl<T: Debug, const N: usize> Debug for PriorityQueue<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[")?;
        if self.len > 0 {
            write!(f, "{:?}", self.get(0))?;
            for i in 1..self.len {
                write!(f, ", {:?}", self.get(i))?;
            }
        }
        write!(f, "]",)
    }
}
