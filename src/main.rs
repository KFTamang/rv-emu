mod bus;
mod cpu;
mod dram;
use crate::cpu::*;

use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        panic!("Usage: rv-emusimple <filename>")
    }
    let mut file = File::open(&args[1])?;
    let mut code = Vec::new();
    file.read_to_end(&mut code)?;

    let mut cpu = Cpu::new(code);

    while cpu.pc < cpu.dram.len() as u64 {
        let inst = cpu.fetch();
        cpu.pc = cpu.pc + 4;

        cpu.execute(inst);

        cpu.dump_registers();
    }

    Ok(())
}
