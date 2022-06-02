mod bus;
mod cpu;
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
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let mut file = File::open(&cli.bin)?;
    let mut code = Vec::new();
    file.read_to_end(&mut code)?;

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

        cpu.pc = cpu.pc + 4;

        if cpu.pc == 0 {
            break;
        }

        if reg_dump {
            cpu.dump_registers();
        }

        if let Some(count_max) = cli.count {
            counter = counter + 1;
            if counter == count_max {
                break;
            }
        }
    }

    Ok(())
}
