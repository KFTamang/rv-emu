pub struct Interrupt {
    pub is_pending: bool,
}

impl Interrupt{
    pub fn new() -> Self {
        Self {is_pending: false}
    }

    pub fn get_pending_interrupt(&self) -> Option<u32> {
        Some(0)
    }
}