use std::io;
use std::net::{TcpListener, TcpStream};

use gdbstub::common::Signal;
use gdbstub::target::{Target, TargetResult};
use gdbstub::target::ext::base::BaseOps;
use gdbstub::target::ext::base::singlethread::{
    SingleThreadResumeOps, SingleThreadSingleStepOps
};
use gdbstub::target::ext::base::singlethread::{
    SingleThreadBase, SingleThreadResume, SingleThreadSingleStep
};
use gdbstub::target::ext::breakpoints::{Breakpoints, SwBreakpoint};
use gdbstub::target::ext::breakpoints::{BreakpointsOps, SwBreakpointOps};

use gdbstub::conn::{Connection, ConnectionExt}; // note the use of `ConnectionExt`
use gdbstub::stub::{run_blocking, DisconnectReason, GdbStub};
use gdbstub::stub::SingleThreadStopReason;

enum MyTargetEvent {
    IncomingData,
    StopReason(SingleThreadStopReason<u32>),
}

pub struct MyTarget;

impl MyTarget{
    pub fn new() -> MyTarget {
        Self
    }

    fn run_and_check_for_incoming_data(
        &mut self,
        conn: &mut impl Connection
    ) -> MyTargetEvent { todo!() }

    fn stop_in_response_to_ctrl_c_interrupt(
        &mut self
    ) -> Result<(), ()> {
        Ok(())
    }

}

impl Target for MyTarget {
    type Error = ();
    type Arch = gdbstub_arch::arm::Armv4t; // as an example

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

impl SingleThreadBase for MyTarget {
    fn read_registers(
        &mut self,
        regs: &mut gdbstub_arch::arm::reg::ArmCoreRegs,
    ) -> TargetResult<(), Self> { todo!() }

    fn write_registers(
        &mut self,
        regs: &gdbstub_arch::arm::reg::ArmCoreRegs
    ) -> TargetResult<(), Self> { todo!() }

    fn read_addrs(
        &mut self,
        start_addr: u32,
        data: &mut [u8],
    ) -> TargetResult<usize, Self> { todo!() }

    fn write_addrs(
        &mut self,
        start_addr: u32,
        data: &[u8],
    ) -> TargetResult<(), Self> { todo!() }

    // most targets will want to support at resumption as well...

    #[inline(always)]
    fn support_resume(&mut self) -> Option<SingleThreadResumeOps<Self>> {
        Some(self)
    }
}

impl SingleThreadResume for MyTarget {
    fn resume(
        &mut self,
        signal: Option<Signal>,
    ) -> Result<(), Self::Error> { todo!() }

    // ...and if the target supports resumption, it'll likely want to support
    // single-step resume as well

    #[inline(always)]
    fn support_single_step(
        &mut self
    ) -> Option<SingleThreadSingleStepOps<'_, Self>> {
        Some(self)
    }
}

impl SingleThreadSingleStep for MyTarget {
    fn step(
        &mut self,
        signal: Option<Signal>,
    ) -> Result<(), Self::Error> { todo!() }
}

impl Breakpoints for MyTarget {
    // there are several kinds of breakpoints - this target uses software breakpoints
    #[inline(always)]
    fn support_sw_breakpoint(&mut self) -> Option<SwBreakpointOps<Self>> {
        Some(self)
    }
}

impl SwBreakpoint for MyTarget {
    fn add_sw_breakpoint(
        &mut self,
        addr: u32,
        kind: gdbstub_arch::arm::ArmBreakpointKind,
    ) -> TargetResult<bool, Self> { todo!() }

    fn remove_sw_breakpoint(
        &mut self,
        addr: u32,
        kind: gdbstub_arch::arm::ArmBreakpointKind,
    ) -> TargetResult<bool, Self> { todo!() }
}

pub fn wait_for_gdb_connection(port: u16) -> io::Result<TcpStream> {
    let sockaddr = format!("localhost:{}", port);
    eprintln!("Waiting for a GDB connection on {:?}...", sockaddr);
    let sock = TcpListener::bind(sockaddr)?;
    let (stream, addr) = sock.accept()?;

    // Blocks until a GDB client connects via TCP.
    // i.e: Running `target remote localhost:<port>` from the GDB prompt.

    eprintln!("Debugger connected from {}", addr);
    Ok(stream) // `TcpStream` implements `gdbstub::Connection`
}


enum MyGdbBlockingEventLoop {}

// The `run_blocking::BlockingEventLoop` groups together various callbacks
// the `GdbStub::run_blocking` event loop requires you to implement.
impl run_blocking::BlockingEventLoop for MyGdbBlockingEventLoop {
    type Target = MyTarget;
    type Connection = Box<dyn ConnectionExt<Error = std::io::Error>>;

    // or MultiThreadStopReason on multi threaded targets
    type StopReason = SingleThreadStopReason<u32>;

    // Invoked immediately after the target's `resume` method has been
    // called. The implementation should block until either the target
    // reports a stop reason, or if new data was sent over the connection.
    fn wait_for_stop_reason(
        target: &mut MyTarget,
        conn: &mut Self::Connection,
    ) -> Result<
        run_blocking::Event<SingleThreadStopReason<u32>>,
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
        let event = match target.run_and_check_for_incoming_data(conn) {
            MyTargetEvent::IncomingData => {
                let byte = conn
                    .read() // method provided by the `ConnectionExt` trait
                    .map_err(run_blocking::WaitForStopReasonError::Connection)?;

                run_blocking::Event::IncomingData(byte)
            }
            MyTargetEvent::StopReason(reason) => {
                run_blocking::Event::TargetStopped(reason)
            }
        };

        Ok(event)
    }

    // Invoked when the GDB client sends a Ctrl-C interrupt.
    fn on_interrupt(
        target: &mut MyTarget,
    ) -> Result<Option<SingleThreadStopReason<u32>>, <MyTarget as Target>::Error> {
        // notify the target that a ctrl-c interrupt has occurred.
        target.stop_in_response_to_ctrl_c_interrupt()?;

        // a pretty typical stop reason in response to a Ctrl-C interrupt is to
        // report a "Signal::SIGINT".
        Ok(Some(SingleThreadStopReason::Signal(Signal::SIGINT).into()))
    }
}

fn gdb_event_loop_thread(
    debugger: GdbStub<MyTarget, Box<dyn ConnectionExt<Error = std::io::Error>>>,
    mut target: MyTarget
) {
    match debugger.run_blocking::<MyGdbBlockingEventLoop>(&mut target) {
        Ok(disconnect_reason) => match disconnect_reason {
            DisconnectReason::Disconnect => {
                println!("Client disconnected")
            }
            DisconnectReason::TargetExited(code) => {
                println!("Target exited with code {}", code)
            }
            DisconnectReason::TargetTerminated(sig) => {
                println!("Target terminated with signal {}", sig)
            }
            DisconnectReason::Kill => println!("GDB sent a kill command"),
        },
        Err(e) => {
            if e.is_target_error() {
                println!(
                    "target encountered a fatal error: {:?}",
                    e.into_target_error().unwrap()
                )
            } else if e.is_connection_error() {
                let (e, kind) = e.into_connection_error().unwrap();
                println!("connection error: {:?} - {:?}", kind, e,)
            } else {
                println!("gdbstub encountered a fatal error: {:?}", e)
            }
        }
    }
}
