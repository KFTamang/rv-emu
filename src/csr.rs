pub struct Csr {
    csr: [u64; 4096],
}

pub const MSTATUS: usize = 0x300;
pub const SSTATUS: usize = 0x100;
pub const MIE: usize = 0x304;
pub const MIP: usize = 0x344;
const BIT_SXL: u64 = 0b11 << 34;
const BIT_TSR: u64 = 0b1 << 22;
const BIT_TW: u64 = 0b1 << 21;
const BIT_TVM: u64 = 0b1 << 20;
const BIT_MPRV: u64 = 0b1 << 17;
const BIT_MPP: u64 = 0b11 << 11;
const BIT_MPIE: u64 = 0b1 << 7;
const BIT_MIE: u64 = 0b1 << 3;
const SSTATUS_MASK: u64 = !(BIT_SXL | BIT_TSR | BIT_TSR | BIT_TW | BIT_TVM | BIT_MPRV | BIT_MPP | BIT_MPIE | BIT_MIE);

impl Csr {
    pub fn new() -> Self {
        Self { csr: [0; 4096] }
    }

    pub fn load_csrs(&self, addr: usize) -> u64 {
        if addr == SSTATUS {
            self.csr[MSTATUS] & SSTATUS_MASK
        } else {
            self.csr[addr]
        }
    }

    pub fn store_csrs(&mut self, addr: usize, val: u64) {
        if addr == SSTATUS {
            self.csr[MSTATUS] = val & SSTATUS_MASK;
        } else {
            self.csr[addr] = val;
        }
    }

    pub fn mstatus_mie(&self) -> bool {
        (self.csr[MSTATUS] & BIT_MIE) != 0
    }

    pub fn mie(&self) -> u64 {
        self.csr[MIE]
    }

    pub fn mip(&self) -> u64 {
        self.csr[MIP]
    }
}
