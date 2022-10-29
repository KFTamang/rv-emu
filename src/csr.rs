pub struct Csr {
    csr: [u64; 4096],
}

pub const MSTATUS: usize = 0x300;
pub const SSTATUS: usize = 0x100;
pub const MIE: usize = 0x304;
pub const MTVEC: usize = 0x305;
pub const MEPC: usize = 0x341;
pub const MCAUSE: usize = 0x342;
pub const MIP: usize = 0x344;

pub const BIT_SXL: u64 = 34;
pub const BIT_TSR: u64 = 22;
pub const BIT_TW: u64 = 21;
pub const BIT_TVM: u64 = 20;
pub const BIT_MPRV: u64 = 17;
pub const BIT_MPP: u64 = 11;
pub const BIT_MPIE: u64 = 7;
pub const BIT_MIE: u64 = 3;

pub const MASK_SXL: u64 = 0b11 << BIT_SXL;
pub const MASK_TSR: u64 = 0b1 << BIT_TSR;
pub const MASK_TW: u64 = 0b1 << BIT_TW;
pub const MASK_TVM: u64 = 0b1 << BIT_TVM;
pub const MASK_MPRV: u64 = 0b1 << BIT_MPRV;
pub const MASK_MPP: u64 = 0b11 << BIT_MPP;
pub const MASK_MPIE: u64 = 0b1 << BIT_MPIE;
pub const MASK_MIE: u64 = 0b1 << BIT_MIE;
const SSTATUS_MASK: u64 = !(MASK_SXL
    | MASK_TSR
    | MASK_TSR
    | MASK_TW
    | MASK_TVM
    | MASK_MPRV
    | MASK_MPP
    | MASK_MPIE
    | MASK_MIE);

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
        (self.csr[MSTATUS] & MASK_MIE) != 0
    }

    fn set_mstatus_bit(&mut self, val: u32, mask: u64, bit: u64) {
        let mut current = self.csr[MSTATUS];
        current &= !mask;
        current |= ((val as u64) << bit) & mask;
        self.csr[MSTATUS] = current;
    }

    pub fn set_mstatus_mpp(&mut self, val: u32) {
        self.set_mstatus_bit(val, MASK_MPP, BIT_MPP);
    }

    pub fn set_mstatus_mpie(&mut self, val: u32) {
        self.set_mstatus_bit(val, MASK_MPIE, BIT_MPIE);
    }

    pub fn set_mstatus_mie(&mut self, val: u32) {
        self.set_mstatus_bit(val, MASK_MIE, BIT_MIE);
    }

    pub fn mie(&self) -> u64 {
        self.csr[MIE]
    }

    pub fn mip(&self) -> u64 {
        self.csr[MIP]
    }

    pub fn set_bit_in_mip(&mut self, val: u32) {
        self.csr[MIP] |= 1u64 << val;
    }
}
