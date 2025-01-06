mod bus;
mod clint;
mod cpu;
mod csr;
mod dram;
mod interrupt;
mod plic;
mod uart;
mod virtio;
mod debugger;
mod emu;
use clap::Parser; // command-line option parser

use std::convert::TryInto;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use crate::debugger::{wait_for_gdb_connection, MyGdbBlockingEventLoop};
use crate::emu::Emu;
use gdbstub::stub::GdbStub;
use gdbstub::stub::DisconnectReason;
use gdbstub::conn::ConnectionExt;
use env_logger;
use log::{error, info};

/// Search for a pattern in a file and display the lines that contain it.
/// c.f. https://rust-cli.github.io/book/tutorial/cli-args.html
#[derive(Parser)]
struct Cli {
    /// The path to the file to read
    bin: std::path::PathBuf,
    #[clap(short, long)]
    dump: Option<i64>,
    #[clap(short, long)]
    count: Option<i64>,
    #[clap(short, long, parse(from_flag))]
    elf: bool,
    #[clap(long)]
    base_addr: Option<usize>,
    #[clap(short, long)]
    output: Option<std::path::PathBuf>,
    #[clap(long)]
    loop_on: bool,
    #[clap(long)]
    gdb: bool,
}

fn main() -> io::Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let mut file = File::open(&cli.bin)?;
    let mut code = Vec::new();
    let mut entry_address = 0 as u64;
    let base_addr = cli.base_addr.unwrap_or(0) as u64;
    let logger = io::BufWriter::new(match cli.output {
        Some(path) => Box::new(File::create(&path).unwrap()) as Box<dyn Write>,
        None => Box::new(io::stdout()) as Box<dyn Write>,
    });

    if cli.elf != false {
        entry_address = load_elf(&mut code, &mut file, base_addr as usize).unwrap();
    } else {
        file.read_to_end(&mut code)?;
    }

    let reg_dump_count = cli.dump.unwrap_or(0);
    let mut counter = cli.count.unwrap_or(1);


    if cli.gdb {
        info!("GDB enabled");
        // Establish a `Connection`
        let connection: Box<dyn ConnectionExt<Error = std::io::Error>> = Box::new(wait_for_gdb_connection(9001)?);

        // Create a new `gdbstub::GdbStub` using the established `Connection`.
        let debugger = GdbStub::new(connection);

        let mut emu = Emu::new(code, base_addr, reg_dump_count as u64, logger);
        emu.set_entry_point(entry_address);
        
        match debugger.run_blocking::<MyGdbBlockingEventLoop>(&mut emu) {
            Ok(disconnect_reason) => match disconnect_reason {
                DisconnectReason::Disconnect => {
                    info!("GDB client has disconnected. Running to completion...");
                    while emu.step() != Some(emu::Event::Halted) {}
                }
                DisconnectReason::TargetExited(code) => {
                    info!("Target exited with code {}!", code)
                }
                DisconnectReason::TargetTerminated(sig) => {
                    info!("Target terminated with signal {}!", sig)
                }
                DisconnectReason::Kill => info!("GDB sent a kill command!"),
            },
            Err(e) => {
                if e.is_target_error() {
                    error!(
                        "target encountered a fatal error: {:?}",
                        e.into_target_error().unwrap()
                    )
                } else if e.is_connection_error() {
                    let (e, kind) = e.into_connection_error().unwrap();
                    error!("connection error: {:?} - {:?}", kind, e,)
                } else {
                    error!("gdbstub encountered a fatal error: {:?}", e)
                }
            }
        }
    } else {
        info!("No GDB");
        let mut emu = Emu::new(code, base_addr, reg_dump_count as u64, logger);
        emu.set_entry_point(entry_address);
        while counter != 0 {
            if emu.step() == Some(emu::Event::Halted) {
                info!("Halted");
                break;
            }
            if !cli.loop_on && counter > 0 {
                counter -= 1;
            }
        }
    }

    // cpu.bus.dump("log/memory.dump");

    Ok(())
}


// fn free_run(cpu: &Cpu, mut poll_incoming_data: impl FnMut() -> bool) -> RunEvent {
//     match exec_mode {
//         ExecMode::Step => RunEvent::Event(step(&cpu).unwrap()),
//         ExecMode::Continue => {
//             let mut cycles = 0;
//             loop {
//                 if cycles % 1024 == 0 {
//                     // poll for incoming data
//                     if poll_incominng_data() {
//                         break RunEvent::IncomingData;
//                     }
//                 }
//                 cycles += 1;

//                 if let Some(event) = step(&cpu) {
//                     break RunEvent::Event(event);
//                 };
//             }
//         }
//     }
// }



pub const E_ENTRY_POS: usize = 0x18; // 64bit
pub const PH_POS: usize = 0x20; // 64bit
pub const PH_ENTRIES_POS: usize = 0x38; // 64bit
pub const PH_ENTRY_SIZE_POS: usize = 0x36; // 64bit

fn u8_slice_to_u16(barry: &[u8]) -> u16 {
    u16::from_le_bytes(barry.try_into().expect("slice with incorrect length"))
}

fn u8_slice_to_u32(barry: &[u8]) -> u32 {
    u32::from_le_bytes(barry.try_into().expect("slice with incorrect length"))
}

fn u8_slice_to_u64(barry: &[u8]) -> u64 {
    u64::from_le_bytes(barry.try_into().expect("slice with incorrect length"))
}

fn load_elf(code: &mut Vec<u8>, file: &mut File, base_addr: usize) -> io::Result<u64> {
    let mut elf = Vec::new();
    file.read_to_end(&mut elf)?;
    let entry = u8_slice_to_u64(&elf[E_ENTRY_POS..E_ENTRY_POS + 8]) as u64;
    let ph_offset = u8_slice_to_u64(&elf[PH_POS..PH_POS + 8]) as usize;
    let ph_entries = u8_slice_to_u16(&elf[PH_ENTRIES_POS..PH_ENTRIES_POS + 2]) as usize;
    let ph_entry_size = u8_slice_to_u16(&elf[PH_ENTRY_SIZE_POS..PH_ENTRY_SIZE_POS + 2]) as usize;
    info!(
        "Prog Header Entries:{}, Offset:{:>#x}, size:{:>#x}, entry:{:>#x}",
        ph_entries, ph_offset, ph_entry_size, entry
    );

    for entry in 0..ph_entries {
        let entry_offset = ph_offset + entry * ph_entry_size;
        let p_type_offset = entry_offset + 0x0;
        let va_offset = entry_offset + 0x10;
        let segment_offset = entry_offset + 0x8;
        let filesize_offset = entry_offset + 0x20;
        let memsize_offset = entry_offset + 0x28;
        let p_type = u8_slice_to_u32(&elf[p_type_offset..p_type_offset + 4]) as usize;
        let segment = u8_slice_to_u64(&elf[segment_offset..segment_offset + 8]) as usize;
        let va = u8_slice_to_u64(&elf[va_offset..va_offset + 8]) as usize;
        let filesize = u8_slice_to_u64(&elf[filesize_offset..filesize_offset + 8]) as usize;
        let memsize = u8_slice_to_u64(&elf[memsize_offset..memsize_offset + 8]) as usize;
        info!("Offset:{:>#x}, Entry:{}, segment offset: {:>#x}, va:{:>#x}, filesize:{:>#x}, memsize:{:>#x}",
                 entry_offset, entry, segment, va, filesize, memsize);
        info!("Code length: {}", code.len());
        if p_type != 0x1 {
            continue;
        }
        if base_addr > va {
            panic!(
                "Base address {:>#x} is larger than virtual address {:>#x}\n",
                base_addr, va
            );
        }
        if code.len() <= (va - base_addr) {
            code.extend(vec![0; va - base_addr - code.len()].iter());
            // extend for .text and .data sections
            code.extend(&elf[segment..segment + filesize]);
            // extend for .bss section, filling with zeros
            code.extend(
                std::iter::repeat(0)
                    .take(memsize - filesize)
                    .collect::<Vec<u8>>(),
            );
        } else if code.len() > va - base_addr + memsize {
            code[va..va + memsize].copy_from_slice(&elf[segment..segment + memsize]);
        } else {
            panic!("Code must have been loaded wrong!");
        }
    }
    Ok(entry)
}
