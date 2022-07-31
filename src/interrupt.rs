use crate::csr::*;

pub struct Interrupt {
    pending_interrupt: Option<u32>,
}

impl Interrupt {
    pub fn new() -> Self {
        Self {
            pending_interrupt: None,
        }
    }

    pub fn get_pending_interrupt(&self) -> Option<u32> {
        self.pending_interrupt
    }

    pub fn set_pending_interrupt(&mut self, csr: &mut Csr, i: u32) {
        self.pending_interrupt = Some(i);
    }
}
