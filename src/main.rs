mod bus;
mod cpu;
mod csr;
mod dram;
mod interrupt;
use crate::cpu::*;
use clap::Parser; // command-line option parser

use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::convert::TryInto;


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
        load_elf(&mut code, &mut file)?;
    } else {
        file.read_to_end(&mut code)?;
    }

    let reg_dump = cli.dump > 0;
    let mut counter = 0;

    let mut cpu = Cpu::new(code);

    loop {
        cpu.process_interrupt();

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

pub const PH_POS: usize = 0x20; // 64bit
pub const PH_ENTRIES_POS: usize = 0x38; // 64bit
pub const PH_ENTRY_SIZE_POS: usize = 0x36; // 64bit

fn u8_slice_to_u16(barry: &[u8]) -> u16 {
    u16::from_le_bytes(barry.try_into().expect("slice with incorrect length"))
}

fn u8_slice_to_u64(barry: &[u8]) -> u64 {
    u64::from_le_bytes(barry.try_into().expect("slice with incorrect length"))
}

fn load_elf(code: &mut Vec<u8>, file: &mut File) -> io::Result<()>{
    let mut elf = Vec::new();
    file.read_to_end(&mut elf)?;
    let ph_offset = u8_slice_to_u64(&elf[PH_POS .. PH_POS+8]) as usize;
    let ph_entries = u8_slice_to_u16(&elf[PH_ENTRIES_POS..PH_ENTRIES_POS+2]) as usize;
    let ph_entry_size = u8_slice_to_u16(&elf[PH_ENTRY_SIZE_POS..PH_ENTRY_SIZE_POS+2]) as usize;

    println!("Prog Header Entries:{}, Offset:{:>#x}, size:{:>#x}", ph_entries, ph_offset, ph_entry_size);

    for entry in 0..ph_entries {
        let entry_offset = ph_offset + entry * ph_entry_size;
        let va_offset = entry_offset + 0x10;
        let segment_offset = entry_offset + 0x8;
        let filesize_offset = entry_offset + 0x20;
        let memsize_offset = entry_offset + 0x28;
        let segment = u8_slice_to_u64(&elf[segment_offset..segment_offset+8])as usize;
        let va = u8_slice_to_u64(&elf[va_offset..va_offset+8]) as usize;
        let filesize = u8_slice_to_u64(&elf[filesize_offset..filesize_offset+8]) as usize;
        let memsize = u8_slice_to_u64(&elf[memsize_offset..memsize_offset+8]) as usize;
        println!("Offset:{:>#x}, Entry:{}, segment offset: {:>#x}, va:{:>#x}, filesize:{:>#x}, memsize:{:>#x}",
                 entry_offset, entry, segment, va, filesize, memsize);
        println!("Code length: {}", code.len());
        if code.len() <= va {
            code.extend(vec![0; va - code.len()].iter());
            code.extend(&elf[segment..segment+filesize]);
        } else if code.len() > va + filesize {
            code[va..va+filesize].copy_from_slice(&elf[segment..segment+filesize]);
        } else {
            panic!("Code must have been loaded wrong!");
        }
    }
    Ok(())
}
