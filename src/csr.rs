pub struct Csr {
    csr: [u64; 4096],
}

pub const MSTATUS: usize = 0x300;
pub const SSTATUS: usize = 0x100;
const SXL: u64 = 0b11 << 34;
const TSR: u64 = 0b1 << 22;
const TW: u64 = 0b1 << 21;
const TVM: u64 = 0b1 << 20;
const MPRV: u64 = 0b1 << 17;
const MPP: u64 = 0b11 << 11;
const MPIE: u64 = 0b1 << 7;
const MIE: u64 = 0b1 << 3;
const SSTATUS_MASK: u64 = !(SXL | TSR | TSR | TW | TVM | MPRV | MPP | MPIE | MIE);

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
}
