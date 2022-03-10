use riscv::register::sstatus::{self, Sstatus, SPP};

#[repr(C)]
pub struct TrapContext {
    x: [usize; 32],
    sstatus: Sstatus,
    pub sepc: usize,
}

impl TrapContext {
    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
        };
        cx.x[2] = sp;
        cx
    }

    pub fn ecall(&mut self) {
        self.sepc += 4;
        self.x[10] =
            crate::syscall::syscall(self.x[17], [self.x[10], self.x[11], self.x[12]]) as usize;
    }
}
