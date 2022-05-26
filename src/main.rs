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

    if args.len() < 2 {
        panic!("Usage: rv-emusimple <filename>")
    }
    let reg_dump = if (args.len() == 3) && (args[2] == "--dump") {1} else {0};
    let mut file = File::open(&args[1])?;
    let mut code = Vec::new();
    file.read_to_end(&mut code)?;

    let mut cpu = Cpu::new(code);

    loop {
        let inst = match cpu.fetch(){
            Ok(inst) => inst,
            Err(_) => break,
        };

        match cpu.execute(inst as u32){
            Ok(_) => {},
            Err(_) => break,
        };

        cpu.pc = cpu.pc + 4;

        if cpu.pc == 0{
            break;
        }

        if reg_dump != 0 {
            cpu.dump_registers();
        }
    }

    Ok(())
}
