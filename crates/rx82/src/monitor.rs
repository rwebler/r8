//! Instruction-level monitor with guest console transport and breakpoints.
#![allow(
    clippy::arbitrary_source_item_ordering,
    reason = "group command handling and execution"
)]
extern crate alloc;
use crate::{console::ConsoleState, state::State::FetchOpcode, system::System};
use alloc::{collections::BTreeSet, rc::Rc};
use anyhow::{Context as _, Result, bail, ensure};
use core::{cell::RefCell, fmt::Write as _};
use std::io::{BufRead, Write, stdin, stdout};

const BANNER: &str = "RMON v1.0 (C) 1977 Solid State Technologies, Inc.";
const USAGE: &str = "Commands (addresses are hexadecimal):
B [<address>] = Add breakpoint, or list breakpoints
BC [<address>]= Remove breakpoint, or clear all breakpoints
G [<address>] = Go until breakpoint, halt, or guest input
H             = Help
I [<text>]    = Queue one guest input line; use G to resume
M [<address>] = Hex and ASCII memory dump (128 bytes)
S [<address>] = Single step (ignores breakpoint on this instruction)
Q             = Quit
Enter         = Repeat last G, M, or S without its address";

/// A parsed monitor command.
#[derive(Clone)]
pub enum Command {
    /// Add or list breakpoints.
    Breakpoint(Option<u16>),
    /// Remove one breakpoint, or clear all.
    ClearBreakpoint(Option<u16>),
    /// Continue execution.
    Go(Option<u16>),
    /// Show commands.
    Help,
    /// Send one guest input line.
    Input(String),
    /// Dump memory.
    Memory(Option<u16>),
    /// Quit the monitor.
    Quit,
    /// Execute one instruction.
    Step(Option<u16>),
}

/// Why execution returned to the monitor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    /// Stopped before executing the instruction at this address.
    Breakpoint(u16),
    /// The guest executed HALT.
    Halted,
    /// One instruction completed.
    Stepped,
    /// The guest needs input from its console.
    WaitingForInput,
}

/// The interactive CLI system monitor.
#[derive(Default)]
pub struct Monitor {
    /// Instruction addresses at which continuous execution stops.
    pub breakpoints: BTreeSet<u16>,
    /// Optional console belonging to this machine.
    pub console: Option<Rc<RefCell<ConsoleState>>>,
    /// Next memory dump address.
    pub last_addr: Option<u16>,
    /// Last repeatable command, with its address removed.
    pub last_cmd: Option<Command>,
    /// Most recent execution stop, used to continue past a breakpoint once.
    pub last_stop: Option<StopReason>,
    /// Single-step mode.
    pub step: bool,
    /// The running system.
    pub sys: System,
}

impl Monitor {
    /// Runs until a breakpoint, HALT, or guest input request.
    /// # Errors
    /// Returns console output errors.
    pub fn go(&mut self, addr: Option<u16>) -> Result<StopReason> {
        self.step = false;
        self.run(addr)
    }

    /// Prints command help.
    pub fn help(&self) {
        println!("{USAGE}");
    }

    /// Opens the monitor at user RAM.
    /// # Errors
    /// Returns terminal I/O errors.
    pub fn interact(&mut self) -> Result<()> {
        self.sys.cpu.pc = 0x0100;
        self.interact_at_current_pc()
    }

    /// Opens the monitor at the current instruction boundary.
    /// # Errors
    /// Returns terminal I/O errors.
    pub fn interact_at_current_pc(&mut self) -> Result<()> {
        self.interact_with(&mut stdin().lock(), &mut stdout().lock())
    }

    /// Opens the monitor with injectable terminal streams. EOF exits cleanly.
    /// # Errors
    /// Returns terminal I/O errors. Invalid commands are reported and ignored.
    pub fn interact_with(
        &mut self,
        input: &mut impl BufRead,
        output: &mut impl Write,
    ) -> Result<()> {
        self.step = true;
        self.last_cmd = Some(Command::Step(None));
        writeln!(output, "{BANNER}")?;
        self.flush_console(output)?;
        loop {
            writeln!(output, "{}", self.sys.debug_state())?;
            write!(output, "> ")?;
            output.flush()?;
            let mut line = String::new();
            if input.read_line(&mut line)? == 0 {
                return Ok(());
            }
            let command = match parse_command(&line, self.last_cmd.as_ref()) {
                Ok(command) => command,
                Err(error) => {
                    writeln!(output, "? {error}")?;
                    continue;
                }
            };
            match command {
                Command::Quit => return Ok(()),
                Command::Help => writeln!(output, "{USAGE}")?,
                Command::Breakpoint(address) => {
                    if let Some(address) = address {
                        self.breakpoints.insert(address);
                    }
                    if self.breakpoints.is_empty() {
                        writeln!(output, "No breakpoints.")?;
                    }
                    for breakpoint in &self.breakpoints {
                        writeln!(output, "Breakpoint {breakpoint:04X}")?;
                    }
                }
                Command::ClearBreakpoint(address) => {
                    if let Some(address) = address {
                        self.breakpoints.remove(&address);
                    } else {
                        self.breakpoints.clear();
                    }
                    writeln!(output, "Breakpoints updated.")?;
                }
                Command::Input(text) => {
                    if let Some(console) = self.console.as_ref() {
                        let mut state = console.borrow_mut();
                        state.input.extend(text.bytes().chain(*b"\n"));
                        state.waiting = false;
                        state.eof = false;
                    } else {
                        writeln!(output, "? No guest console attached.")?;
                    }
                }
                Command::Memory(address) => {
                    self.last_cmd = Some(Command::Memory(None));
                    write!(output, "{}", self.memory_dump(address))?;
                }
                Command::Go(address) | Command::Step(address) => {
                    self.step = matches!(command, Command::Step(_));
                    self.last_cmd = Some(if self.step {
                        Command::Step(None)
                    } else {
                        Command::Go(None)
                    });
                    let stopped = self.run_with_output(address, output)?;
                    match stopped {
                        StopReason::Breakpoint(breakpoint) => {
                            writeln!(output, "Breakpoint at {breakpoint:04X}")?;
                        }
                        StopReason::Halted => writeln!(output, "Halted.")?,
                        StopReason::Stepped => {}
                        StopReason::WaitingForInput => {
                            writeln!(output, "Guest waiting for input. Use I <text>, then G.")?;
                        }
                    }
                }
            }
        }
    }

    /// Prints 128 bytes of memory in hex and ASCII.
    pub fn memory(&mut self, addr: Option<u16>) {
        print!("{}", self.memory_dump(addr));
    }

    /// Formats a dump and advances the dump cursor. Nonprintable bytes use dots.
    #[must_use]
    #[expect(clippy::unwrap_used, reason = "formatting into a String cannot fail")]
    pub fn memory_dump(&mut self, addr: Option<u16>) -> String {
        let mut base = addr.unwrap_or(self.last_addr.unwrap_or(self.sys.cpu.pc));
        let mut dump = String::new();
        for _ in 0..8_u8 {
            write!(dump, "{base:04X}:").unwrap();
            let mut ascii = String::new();
            for offset in 0..16_u16 {
                let byte = self.sys.peek_mem(base.wrapping_add(offset));
                write!(dump, " {byte:02X}").unwrap();
                ascii.push(if (0x20..=0x7E).contains(&byte) {
                    char::from(byte)
                } else {
                    '.'
                });
            }
            writeln!(dump, "  |{ascii}|").unwrap();
            base = base.wrapping_add(16);
        }
        self.last_addr = Some(base);
        dump
    }

    /// Executes using the terminal for guest output.
    /// # Errors
    /// Returns console output errors.
    pub fn run(&mut self, addr: Option<u16>) -> Result<StopReason> {
        self.run_with_output(addr, &mut stdout().lock())
    }

    /// Runs at instruction boundaries, delivering console output as it is emitted.
    /// # Errors
    /// Returns output errors or an error if called mid-instruction.
    pub fn run_with_output(
        &mut self,
        addr: Option<u16>,
        output: &mut impl Write,
    ) -> Result<StopReason> {
        ensure!(
            self.sys.cpu.state == FetchOpcode,
            "monitor execution requires an instruction boundary"
        );
        let mut skip_breakpoint =
            addr.is_none() && self.last_stop == Some(StopReason::Breakpoint(self.sys.cpu.pc));
        if let Some(addr) = addr {
            self.sys.cpu.pc = addr;
        }
        self.sys.cpu.halt = false;
        self.last_stop = None;
        self.flush_console(output)?;
        let reason = loop {
            if self.sys.cpu.state == FetchOpcode {
                if !self.step && !skip_breakpoint && self.breakpoints.contains(&self.sys.cpu.pc) {
                    break StopReason::Breakpoint(self.sys.cpu.pc);
                }
                skip_breakpoint = false;
                if !self.step
                    && self.console.as_ref().is_some_and(|console| {
                        let state = console.borrow();
                        state.waiting && state.input.is_empty() && !state.eof
                    })
                {
                    break StopReason::WaitingForInput;
                }
            }
            self.sys.tick();
            self.flush_console(output)?;
            if self.sys.cpu.state == FetchOpcode {
                if self.sys.cpu.halt {
                    break StopReason::Halted;
                }
                if self.step {
                    break StopReason::Stepped;
                }
            }
        };
        // A store enters FetchOpcode when its bus write is issued, one cycle
        // before devices consume it. Commit it before inspection can replace
        // the bus request, without executing the next CPU instruction.
        if self.sys.bus.mem && self.sys.bus.write {
            let halted = self.sys.cpu.halt;
            self.sys.cpu.halt = true;
            self.sys.tick();
            self.sys.cpu.halt = halted;
            self.flush_console(output)?;
        }
        self.last_addr = Some(self.sys.cpu.pc);
        self.last_stop = Some(reason);
        Ok(reason)
    }

    fn flush_console(&mut self, output: &mut impl Write) -> Result<()> {
        if let Some(console) = self.console.as_ref() {
            let mut state = console.borrow_mut();
            if !state.output.is_empty() {
                output.write_all(&state.output)?;
                output.flush()?;
                state.output.clear();
            }
        }
        Ok(())
    }

    /// Loads machine code into RAM and runs or monitors it.
    /// # Errors
    /// Returns memory loading or terminal I/O errors.
    pub fn run_program(&mut self, program: &[u8]) -> Result<()> {
        self.sys.mem.load(0x0100, program)?;
        self.sys.cpu.pc = 0x0100;
        if self.step {
            self.interact()
        } else {
            self.run(None)?;
            Ok(())
        }
    }

    /// Executes one instruction, even if it has a breakpoint.
    /// # Errors
    /// Returns console output errors.
    pub fn step(&mut self, addr: Option<u16>) -> Result<StopReason> {
        self.step = true;
        self.run(addr)
    }
}

#[cfg_attr(
    not(test),
    expect(
        clippy::single_call_fn,
        reason = "keep command validation separate from effects"
    )
)]
fn parse_command(line: &str, previous: Option<&Command>) -> Result<Command> {
    let line = line.trim_end_matches(['\r', '\n']).trim_start();
    if line.trim().is_empty() {
        return Ok(previous.cloned().unwrap_or(Command::Step(None)));
    }
    let (name, argument) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    if name.eq_ignore_ascii_case("I") {
        return Ok(Command::Input(argument.to_owned()));
    }
    let argument = argument.trim();
    let address = if argument.is_empty() {
        None
    } else {
        Some(
            u16::from_str_radix(
                argument
                    .strip_prefix("0x")
                    .or_else(|| argument.strip_prefix("0X"))
                    .unwrap_or(argument),
                16,
            )
            .context("expected a hexadecimal address from 0000 to FFFF")?,
        )
    };
    match name.to_ascii_uppercase().as_str() {
        "B" => Ok(Command::Breakpoint(address)),
        "BC" => Ok(Command::ClearBreakpoint(address)),
        "G" => Ok(Command::Go(address)),
        "M" => Ok(Command::Memory(address)),
        "S" => Ok(Command::Step(address)),
        "H" if address.is_none() => Ok(Command::Help),
        "Q" if address.is_none() => Ok(Command::Quit),
        _ => bail!("unknown command or unexpected argument; type H for help"),
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "tests")]
mod tests {
    use super::*;
    use crate::console::Console;
    use r8cpu::regs::Reg::A;

    fn machine(source: &str) -> Monitor {
        let mut monitor = Monitor::default();
        monitor.sys.turbo = true;
        monitor
            .sys
            .mem
            .load(0x0100, &r8asm::assemble(source).unwrap())
            .unwrap();
        monitor.sys.cpu.pc = 0x0100;
        monitor
    }

    #[test]
    fn breakpoints_stop_before_execution_and_resume_once() {
        let mut monitor = machine("inc a\ninc a\nhalt");
        monitor.breakpoints.extend([0x0100, 0x0101]);
        let mut output = Vec::new();
        assert_eq!(
            monitor.run_with_output(None, &mut output).unwrap(),
            StopReason::Breakpoint(0x0100)
        );
        assert_eq!(monitor.sys.cpu.regs.get(A), 0);
        assert_eq!(
            monitor.run_with_output(None, &mut output).unwrap(),
            StopReason::Breakpoint(0x0101)
        );
        assert_eq!(monitor.sys.cpu.regs.get(A), 1);
        assert!(monitor.memory_dump(Some(0x0100)).starts_with("0100:"));
        assert_eq!(
            monitor.run_with_output(None, &mut output).unwrap(),
            StopReason::Halted
        );
        assert_eq!(monitor.sys.cpu.regs.get(A), 2);
    }

    #[test]
    fn breakpoint_is_rearmed_on_a_self_jump_and_step_ignores_it() {
        let mut monitor = machine("jmp 0x0100");
        monitor.breakpoints.insert(0x0100);
        let mut output = Vec::new();
        for _ in 0..2_u8 {
            assert_eq!(
                monitor.run_with_output(None, &mut output).unwrap(),
                StopReason::Breakpoint(0x0100)
            );
        }
        monitor.step = true;
        assert_eq!(
            monitor.run_with_output(None, &mut output).unwrap(),
            StopReason::Stepped
        );
        assert_eq!(monitor.sys.cpu.pc, 0x0100);
    }

    #[test]
    fn stopped_store_commits_ram_and_console_before_inspection() {
        let mut monitor = machine("ld a, 0x58\nld 0x0200, a\nld 0xFF02, a\nhalt");
        let console = Console::default();
        let shared = Rc::clone(&console.shared);
        monitor.console = Some(Rc::clone(&shared));
        monitor.sys.devices.insert(0, Box::new(console));
        monitor.breakpoints.insert(0x0105);
        let mut output = Vec::new();
        assert_eq!(
            monitor.run_with_output(None, &mut output).unwrap(),
            StopReason::Breakpoint(0x0105)
        );
        assert_eq!(monitor.sys.mem.get(0x0200), b'X');
        assert!(
            monitor
                .memory_dump(Some(0x0200))
                .contains("|X...............|")
        );
        monitor.step = true;
        assert_eq!(
            monitor.run_with_output(None, &mut output).unwrap(),
            StopReason::Stepped
        );
        assert_eq!(output, b"X");
        assert_eq!(shared.borrow().output, Vec::<u8>::new());
        assert_eq!(
            monitor.run_with_output(None, &mut output).unwrap(),
            StopReason::Halted
        );
        assert_eq!(output, b"X");
    }

    #[test]
    fn ascii_dumps_advance_and_wrap() {
        let mut monitor = machine("halt");
        monitor
            .sys
            .mem
            .load(0x0200, &[0, b' ', b'A', b'~', 0x7F, 0xFF])
            .unwrap();
        let dump = monitor.memory_dump(Some(0x0200));
        assert!(dump.starts_with("0200: 00 20 41 7E 7F FF"));
        assert!(dump.contains("|. A~............|"));
        assert_eq!(monitor.last_addr, Some(0x0280));
        assert!(monitor.memory_dump(None).starts_with("0280:"));
        let wrapped = monitor.memory_dump(Some(0xFFF0));
        assert!(wrapped.contains("\n0000:"));
        assert_eq!(monitor.last_addr, Some(0x0070));
    }

    #[test]
    fn commands_validate_addresses_and_handle_eof() {
        assert!(parse_command("G nope", None).is_err());
        assert!(parse_command("B 10000", None).is_err());
        assert!(parse_command("G 100 200", None).is_err());
        assert!(matches!(
            parse_command("B 0xc000", None).unwrap(),
            Command::Breakpoint(Some(0xC000))
        ));
        assert!(
            matches!(parse_command("i Hello world", None).unwrap(), Command::Input(text) if text == "Hello world")
        );
        let mut monitor = machine("inc a\nhalt");
        let mut output = Vec::new();
        monitor
            .interact_with(
                &mut b"B 100\nB\nB 101\nBC 100\nG nope\n".as_slice(),
                &mut output,
            )
            .unwrap();
        assert_eq!(monitor.breakpoints, BTreeSet::from([0x0101]));
        assert_eq!(monitor.sys.cpu.pc, 0x0100);
        assert_eq!(monitor.sys.cpu.regs.get(A), 0);
        monitor
            .interact_with(&mut b"BC\nQ\n".as_slice(), &mut output)
            .unwrap();
        assert!(monitor.breakpoints.is_empty());
    }
}
