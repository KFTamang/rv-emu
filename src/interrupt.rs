pub struct Interrupt {
    pub is_pending: bool,
}

impl Interrupt{
    pub fn new() -> Self {
        Self {is_pending: false}
    } 
}