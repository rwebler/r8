//! Native BASIC ROM runner. The host transports bytes; the R8 interprets BASIC.
extern crate alloc;
use crate::{
    console::{Console, ConsoleState},
    rom::Rom,
    system::System,
};
use alloc::rc::Rc;
use anyhow::Result;
use core::cell::RefCell;
use std::io::{BufRead, Write};

/// Native interpreter assembled from `sys/basic_rom.asm`.
pub const ROM: &[u8] = include_bytes!("../sys/basic_rom.bin");

/// Creates a machine with the BASIC ROM and console and starts its reset sequence.
#[must_use]
pub fn machine() -> (System, Rc<RefCell<ConsoleState>>) {
    let console = Console::default();
    let shared = Rc::clone(&console.shared);
    let mut sys = System {
        turbo: true,
        ..System::default()
    };
    sys.devices.clear();
    sys.devices.push(Box::new(console));
    sys.devices.push(Box::new(Rom {
        start: 0xC000,
        end: 0xFEFF,
        data: ROM.to_vec(),
    }));
    sys.devices.push(Box::new(Rom {
        start: 0xFFFE,
        end: 0xFFFF,
        data: vec![0x00, 0xC0],
    }));
    sys.cpu.reset(&mut sys.bus);
    (sys, shared)
}

/// Boots the native ROM and transports terminal bytes until the guest halts.
/// # Errors
/// Returns host terminal I/O errors.
pub fn interact(
    source: Option<&str>,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<()> {
    let (mut sys, shared) = machine();
    if let Some(source) = source {
        shared
            .borrow_mut()
            .input
            .extend(source.bytes().chain(b"\nRUN\nQUIT\n".iter().copied()));
    }
    while !sys.cpu.halt {
        sys.tick();
        let bytes = core::mem::take(&mut shared.borrow_mut().output);
        if !bytes.is_empty() {
            output.write_all(&bytes)?;
            output.flush()?;
        }
        if shared.borrow().waiting {
            let mut line = String::new();
            let count = input.read_line(&mut line)?;
            let mut state = shared.borrow_mut();
            state.waiting = false;
            state.eof = count == 0;
            if count != 0 {
                if !line.ends_with('\n') {
                    line.push('\n');
                }
                state.input.extend(line.bytes());
            }
        }
    }
    Ok(())
}

/// Opens the monitor with a source file queued as guest keyboard input.
/// # Errors
/// Returns monitor I/O errors.
pub fn debug(source: &str) -> Result<()> {
    let (sys, shared) = machine();
    shared
        .borrow_mut()
        .input
        .extend(source.bytes().chain(b"\nRUN\nQUIT\n".iter().copied()));
    shared.borrow_mut().eof = true;
    let mut monitor = crate::monitor::Monitor {
        sys,
        ..Default::default()
    };
    monitor.interact_at_current_pc()
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "tests")]
mod tests {
    use super::*;
    fn session(source: &str) -> (System, String) {
        let (mut sys, shared) = machine();
        shared.borrow_mut().input.extend(source.bytes());
        shared.borrow_mut().eof = true;
        for _ in 0..5_000_000_u32 {
            if sys.cpu.halt {
                break;
            }
            sys.tick();
        }
        assert!(sys.cpu.halt, "guest did not halt at {:04X}", sys.cpu.pc);
        let output = String::from_utf8(shared.borrow().output.clone()).unwrap();
        (sys, output)
    }
    #[test]
    fn rom_matches_source_and_fits() {
        assert_eq!(
            r8asm::assemble(include_str!("../sys/basic_rom.asm")).unwrap(),
            ROM
        );
        assert!(ROM.len() <= 0x3F00);
    }
    #[test]
    fn native_editor_and_execution_use_guest_ram() {
        let (sys, output) = session("20 PRINT A\n10 LET A=42\n30 END\nLIST\nRUN\nQUIT\n");
        assert!(
            output.contains("10 LET A=42\n20 PRINT A\n30 END\n"),
            "{output}"
        );
        assert!(output.contains("42\n"), "{output}");
        assert_eq!(sys.mem.get(0x0300), 42);
        assert_eq!(sys.mem.get(0x1000), 20);
        assert!(sys.cpu.pc >= 0xC000);
    }
    #[test]
    fn native_edit_delete_and_goto() {
        let (_, output) =
            session("10 PRINT 1\n20 GOTO 40\n30 PRINT 99\n40 PRINT 3\n10 PRINT 2\n30\nRUN\nQUIT\n");
        assert!(output.contains("2\n3\n"), "{output}");
        assert!(!output.contains("99\n"), "{output}");
    }
}
