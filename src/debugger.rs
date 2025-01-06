use std::convert::TryInto;
use std::io;
use std::net::{TcpListener, TcpStream};
use std::ptr::read;
use log::{error, info};

use crate::dram;
use crate::emu::{Emu, Event, ExecMode, RunEvent};

use gdbstub::common::Signal;
use gdbstub::target::ext::base::singlethread::{
    SingleThreadBase, SingleThreadResume, SingleThreadSingleStep,
};
use gdbstub::target::ext::base::singlethread::{SingleThreadResumeOps, SingleThreadSingleStepOps};
use gdbstub::target::ext::base::BaseOps;
use gdbstub::target::ext::breakpoints::{Breakpoints, SwBreakpoint};
use gdbstub::target::ext::breakpoints::{BreakpointsOps, SwBreakpointOps};
use gdbstub::target::{Target, TargetError, TargetResult};

use gdbstub::conn::{Connection, ConnectionExt}; // note the use of `ConnectionExt`
use gdbstub::stub::run_blocking;
use gdbstub::stub::SingleThreadStopReason;

impl Target for Emu {
    type Error = ();
    type Arch = gdbstub_arch::riscv::Riscv64; // as an example

    #[inline(always)]
    fn base_ops(&mut self) -> BaseOps<Self::Arch, Self::Error> {
        BaseOps::SingleThread(self)
    }

    // opt-in to support for setting/removing breakpoints
    #[inline(always)]
    fn support_breakpoints(&mut self) -> Option<BreakpointsOps<Self>> {
        Some(self)
    }
}

impl SingleThreadBase for Emu {
    fn read_registers(
        &mut self,
        regs: &mut gdbstub_arch::riscv::reg::RiscvCoreRegs<u64>,
    ) -> TargetResult<(), Self> {
        regs.x = self.cpu.regs;
        regs.pc = self.cpu.pc;
        Ok(())
    }

    fn write_registers(
        &mut self,
        regs: &gdbstub_arch::riscv::reg::RiscvCoreRegs<u64>,
    ) -> TargetResult<(), Self> {
        self.cpu.regs = regs.x;
        self.cpu.pc = regs.pc;
        Ok(())
    }

    fn read_addrs(&mut self, start_addr: u64, data: &mut [u8]) -> TargetResult<usize, Self> {
        let mut read_size = 0;
        while data.len() - read_size >= 8 {
            // load 64 bytes at a time, copy to data
            if let Ok(source_slice) = self.cpu.bus.load(start_addr + read_size as u64, 64) {
                data[read_size..read_size + 8].copy_from_slice(&source_slice.to_le_bytes());
                read_size += 8;
            } else {
                return Err(TargetError::NonFatal);
            }
        }
        while data.len() - read_size > 0 {
            if let Ok(source_slice) = self.cpu.bus.load(start_addr + read_size as u64, 8) {
                data[read_size] = source_slice as u8;
                read_size += 1;
            } else {
                return Err(TargetError::NonFatal);
            }
        }
        Ok(read_size)
    }

    fn write_addrs(&mut self, start_addr: u64, data: &[u8]) -> TargetResult<(), Self> {
        let mut wrote_size = 0;
        while data.len() - wrote_size >= 8 {
            // convert data[wrote_size..wrote_size + 8] into one integer `data_8byte`
            let data_8byte =
                u64::from_le_bytes(data[wrote_size..wrote_size + 8].try_into().unwrap());
            // store 64 bytes at a time, copy to data
            if let Ok(source_slice) =
                self.cpu
                    .bus
                    .store(start_addr + wrote_size as u64, data_8byte, 64)
            {
                wrote_size += 8;
            } else {
                return Err(TargetError::NonFatal);
            }
        }
        while data.len() - wrote_size > 0 {
            if let Ok(source_slice) =
                self.cpu
                    .bus
                    .store(start_addr + wrote_size as u64, data[wrote_size] as u64, 8)
            {
                wrote_size += 1;
            } else {
                return Err(TargetError::NonFatal);
            }
        }
        Ok(())
    }

    // most targets will want to support at resumption as well...

    #[inline(always)]
    fn support_resume(&mut self) -> Option<SingleThreadResumeOps<Self>> {
        Some(self)
    }
}

impl SingleThreadResume for Emu {
    fn resume(&mut self, signal: Option<Signal>) -> Result<(), Self::Error> {
        self.exec_mode = ExecMode::Continue;
        Ok(())
    }

    // ...and if the target supports resumption, it'll likely want to support
    // single-step resume as well

    #[inline(always)]
    fn support_single_step(&mut self) -> Option<SingleThreadSingleStepOps<'_, Self>> {
        Some(self)
    }
}

impl SingleThreadSingleStep for Emu {
    fn step(&mut self, signal: Option<Signal>) -> Result<(), Self::Error> {
        self.exec_mode = ExecMode::Step;
        Ok(())
    }
}

impl Breakpoints for Emu {
    // there are several kinds of breakpoints - this target uses software breakpoints
    #[inline(always)]
    fn support_sw_breakpoint(&mut self) -> Option<SwBreakpointOps<Self>> {
        Some(self)
    }
}

impl SwBreakpoint for Emu {
    fn add_sw_breakpoint(&mut self, addr: u64, kind: usize) -> TargetResult<bool, Self> {
        todo!()
    }

    fn remove_sw_breakpoint(&mut self, addr: u64, kind: usize) -> TargetResult<bool, Self> {
        todo!()
    }
}

pub fn wait_for_gdb_connection(port: u16) -> io::Result<TcpStream> {
    let sockaddr = format!("0.0.0.0:{}", port);
    info!("Waiting for a GDB connection on {:?}...", sockaddr);
    let sock = TcpListener::bind(sockaddr)?;
    let (stream, addr) = sock.accept()?;

    // Blocks until a GDB client connects via TCP.
    // i.e: Running `target remote localhost:<port>` from the GDB prompt.

    info!("Debugger connected from {}", addr);
    Ok(stream) // `TcpStream` implements `gdbstub::Connection`
}

pub enum MyGdbBlockingEventLoop {}

// The `run_blocking::BlockingEventLoop` groups together various callbacks
// the `GdbStub::run_blocking` event loop requires you to implement.
impl run_blocking::BlockingEventLoop for MyGdbBlockingEventLoop {
    type Target = Emu;
    type Connection = Box<dyn ConnectionExt<Error = std::io::Error>>;

    // or MultiThreadStopReason on multi threaded targets
    type StopReason = SingleThreadStopReason<u64>;

    // Invoked immediately after the target's `resume` method has been
    // called. The implementation should block until either the target
    // reports a stop reason, or if new data was sent over the connection.
    fn wait_for_stop_reason(
        target: &mut Emu,
        conn: &mut Self::Connection,
    ) -> Result<
        run_blocking::Event<SingleThreadStopReason<u64>>,
        run_blocking::WaitForStopReasonError<
            <Self::Target as Target>::Error,
            <Self::Connection as Connection>::Error,
        >,
    > {
        // the specific mechanism to "select" between incoming data and target
        // events will depend on your project's architecture.
        //
        // some examples of how you might implement this method include: `epoll`,
        // `select!` across multiple event channels, periodic polling, etc...
        //
        // in this example, lets assume the target has a magic method that handles
        // this for us.

        let poll_incoming_data = || {
            // gdbstub takes ownership of the underlying connection, so the `borrow_conn`
            // method is used to borrow the underlying connection back from the stub to
            // check for incoming data.
            conn.peek().map(|b| b.is_some()).unwrap_or(true)
        };

        match target.run(poll_incoming_data) {
            RunEvent::IncomingData => {
                let byte = conn
                    .read()
                    .map_err(run_blocking::WaitForStopReasonError::Connection)?;
                Ok(run_blocking::Event::IncomingData(byte))
            }
            RunEvent::Event(event) => {
                // translate emulator stop reason into GDB stop reason
                let stop_reason = match event {
                    Event::DoneStep => SingleThreadStopReason::DoneStep,
                    Event::Halted => SingleThreadStopReason::Terminated(Signal::SIGSTOP),
                    Event::Break => SingleThreadStopReason::SwBreak(()),
                };
                Ok(run_blocking::Event::TargetStopped(stop_reason))
            }
        }
    }

    // Invoked when the GDB client sends a Ctrl-C interrupt.
    fn on_interrupt(
        target: &mut Emu,
    ) -> Result<Option<SingleThreadStopReason<u64>>, <Emu as Target>::Error> {
        // notify the target that a ctrl-c interrupt has occurred.

        // a pretty typical stop reason in response to a Ctrl-C interrupt is to
        // report a "Signal::SIGINT".
        Ok(Some(SingleThreadStopReason::Signal(Signal::SIGINT).into()))
    }
}
