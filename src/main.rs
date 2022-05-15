use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;

// dram memory size, 128MB
pub const DRAM_SIZE: u64 = 1024 * 1024 * 128;

struct Cpu {
    regs: [u64; 32],
    pc: u64,
    dram: Vec<u8>,
}
fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        panic!("Usage: rv-emusimple <filename>")
    }
    let mut file = File::open(&args[1])?;
    let mut code = Vec::new();
    file.read_to_end(&mut code)?;

    let cpu = Cpu::new(code);

    Ok(())
}

impl Cpu {
    fn new(code: Vec<u8>) -> Self {
        let mut regs = [0; 32];
        regs[2] = DRAM_SIZE;
        Self {
            regs,
            pc: 0,
            dram: code,
        }
    }

    fn fetch(&self) -> u32 {
        let index = self.pc as usize;
        return index as u32;
    }
    fn execute(&mut self, inst: u32) {}
}
