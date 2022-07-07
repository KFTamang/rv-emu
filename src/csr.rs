pub struct Csr {
    csr: [u32; 4096],
}

impl Csr {
    pub fn new() -> Self {
        Self { csr: [0; 4096] }
    }

    pub fn load_csrs(&self, addr: usize) -> u32 {
        self.csr[addr]
    }

    pub fn store_csrs(&mut self, addr: usize, val: u32) {
        self.csr[addr] = val;
    }
}
