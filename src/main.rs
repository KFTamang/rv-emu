mod bus;
mod cpu;
mod csr;
mod dram;
use crate::cpu::*;
use clap::Parser; // command-line option parser

use std::fs::File;
use std::io;
use std::io::prelude::*;

/// Search for a pattern in a file and display the lines that contain it.
/// c.f. https://rust-cli.github.io/book/tutorial/cli-args.html
#[derive(Parser)]
struct Cli {
    /// The path to the file to read
    bin: std::path::PathBuf,
    #[clap(short, long, parse(from_occurrences))]
    dump: usize,
    #[clap(short, long)]
    count: Option<u32>,
    #[clap(short, long, parse(from_flag))]
    elf: bool,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let mut file = File::open(&cli.bin)?;
    let mut code = Vec::new();

    if cli.elf != false {
        file.read_to_end(&mut code)?;
    } else {
        load_elf(&mut code, &mut file);
    }

    let reg_dump = cli.dump > 0;
    let mut counter = 0;

    let mut cpu = Cpu::new(code);

    loop {
        let inst = match cpu.fetch() {
            Ok(inst) => inst,
            Err(_) => break,
        };

        match cpu.execute(inst as u32) {
            Ok(_) => {}
            Err(_) => break,
        };
        cpu.regs[0] = 0;

        cpu.pc = cpu.pc.wrapping_add(4);

        if cpu.pc == 0 {
            cpu.dump_registers();
            println!("Program finished!");
            break;
        }

        if reg_dump {
            cpu.dump_registers();
        }

        if let Some(count_max) = cli.count {
            counter = counter + 1;
            if counter == count_max {
                println!("Program readched execution limit.");
                break;
            }
        }
    }

    Ok(())
}

pub const ph_pos: usize = 0x20; // 64bit
pub const ph_entries_pos: usize = 0x38; // 64bit
pub const ph_entry_size_pos: usize = 0x36; // 64bit

fn load_elf(code: &mut Vec<u8>, file: &mut File) {
    let mut elf = Vec::new();
    file.read_to_end(&mut elf);
    let ph_offset = elf[ph_pos] as usize;
    let ph_entries = elf[ph_entries_pos] as usize;
    let ph_entry_size = elf[ph_entry_size_pos] as usize;

    println!("Prog Header Entries:{}", ph_entries);

    for entry in 0..(ph_entries - 1) {
        let va = elf[ph_offset + entry * ph_entry_size];
        println!("Entry:{}, va:{}", entry, va);
    }
}
