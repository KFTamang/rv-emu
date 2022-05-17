use crate::dram::*;

pub struct Bus {
    dram: Dram,
}

impl Bus {
    pub fn new(code: Vec<u8>) -> Bus {
        Self {
            dram: Dram::new(code),
        }
    }
}
