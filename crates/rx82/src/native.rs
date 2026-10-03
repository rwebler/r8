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

/// Creates a machine paused at the BASIC ROM entry point after CPU reset.
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
    sys.devices.push(Box::<crate::files::FilePort>::default());
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
    // Monitor memory reads are safe at instruction boundaries. Complete the
    // reset-vector fetch before exposing this machine to the debugger.
    while sys.cpu.state != crate::state::State::FetchOpcode {
        sys.tick();
    }
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
            .extend(source.bytes().chain(b"\nRUN\n".iter().copied()));
    }
    let mut ran = false;
    while !sys.cpu.halt {
        sys.tick();
        ran |= sys.mem.get(0x0082) != 0;
        let bytes = core::mem::take(&mut shared.borrow_mut().output);
        if !bytes.is_empty() {
            output.write_all(&bytes)?;
            output.flush()?;
        }
        if shared.borrow().waiting {
            if source.is_some() && ran && sys.mem.get(0x0082) == 0 {
                let mut state = shared.borrow_mut();
                state.waiting = false;
                state.input.extend(b"QUIT\n");
                continue;
            }
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
        .extend(source.bytes().chain(b"\nRUN\n".iter().copied()));
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
    fn monitor_inspection_preserves_native_startup() {
        let (sys, shared) = machine();
        shared.borrow_mut().input.extend(b"10 A=42\nRUN\nQUIT\n");
        shared.borrow_mut().eof = true;
        let mut monitor = crate::monitor::Monitor {
            sys,
            ..Default::default()
        };
        assert_eq!(monitor.sys.cpu.pc, 0xC000);
        monitor.sys.debug_print();
        monitor.memory(Some(0x1000));
        assert_eq!(monitor.sys.cpu.pc, 0xC000);
        monitor.step(None);
        monitor.go(None);
        assert_eq!(monitor.sys.mem.get(0x0300), 42);
        assert_eq!(monitor.sys.mem.get(0x1000), 10);
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
    #[test]
    fn native_matches_reference_arithmetic_and_control_flow() {
        for source in [
            "10 PRINT -32768,32767,2+3*4,(2+3)*4,-7/2,-3*-4",
            "10 FOR I=1 TO 2\n20 FOR J=2 TO 1 STEP -1\n30 PRINT I;J\n40 NEXT J\n50 NEXT I",
            "10 FOR I=2 TO 1\n20 PRINT 99\n30 NEXT I\n40 PRINT 7",
            "10 A=1\n20 PRINT A*A\n30 A=A+1\n40 IF A<=3 THEN 20\n50 END",
            "10 FOR I=1 TO 2\n20 GOSUB 100\n30 NEXT I\n40 END\n100 FOR J=1 TO 3\n110 PRINT I;J\n120 RETURN\n130 NEXT J",
            "10 FOR I=1 TO 2\n20 FOR J=1 TO 2\n30 PRINT I\n40 GOTO 60\n50 NEXT J\n60 NEXT I",
        ] {
            let mut reference = crate::basic::Basic::default();
            reference.load(source).unwrap();
            let mut expected = Vec::new();
            reference.run(&mut b"".as_slice(), &mut expected).unwrap();
            let (_, output) = session(&format!(
                "{source}\nPRINT \"BEGIN\"\nRUN\nPRINT \"FINISH\"\nQUIT\n"
            ));
            let body = output
                .split("BEGIN\n> ")
                .nth(1)
                .unwrap()
                .split("> FINISH\n")
                .next()
                .unwrap();
            assert_eq!(body.as_bytes(), expected, "{source}\n{output}");
        }
    }
    #[test]
    fn native_reports_arithmetic_errors_and_recovers() {
        for expression in ["32767+1", "-32768-1", "200*200", "-32768/-1", "--32768"] {
            let (_, output) = session(&format!("10 PRINT {expression}\nRUN\nPRINT 7\nQUIT\n"));
            assert!(output.contains("? INTEGER OVERFLOW IN LINE 10"), "{output}");
            assert!(output.contains("> 7\n"), "{output}");
        }
        let (_, output) = session("10 PRINT 1/0\nRUN\nQUIT\n");
        assert!(output.contains("? DIVISION BY ZERO IN LINE 10"), "{output}");
    }
    #[test]
    fn native_input_and_full_range_line_numbers() {
        let (_, output) =
            session("10 INPUT A\n20 PRINT A\n30 GOTO 65535\n65535 PRINT 7\nRUN\n-32768\nQUIT\n");
        assert!(output.contains("? -32768\n7\n"), "{output}");
    }
    #[test]
    fn native_loop_errors_return_to_prompt() {
        for source in [
            "10 FOR I=1 TO 2 STEP 0\n20 NEXT",
            "10 FOR I=1 TO 2\n20 NEXT J",
            "10 NEXT I",
            "10 FOR I=1 TO 2",
            "10 FOR I=32767 TO 32767\n20 NEXT I",
        ] {
            let (_, output) = session(&format!("{source}\nRUN\nPRINT 7\nQUIT\n"));
            assert!(
                output.contains("? ") && output.contains("> 7\n"),
                "{output}"
            );
        }
    }
    #[test]
    fn native_signed_comparisons() {
        for (comparison, expected) in [
            ("-2 < 1", true),
            ("1 > -2", true),
            ("-2 >= -2", true),
            ("1 <> 2", true),
            ("1 = 2", false),
            ("1 <= -2", false),
        ] {
            let (_, output) = session(&format!("10 IF {comparison} THEN PRINT 77\nRUN\nQUIT\n"));
            assert_eq!(output.contains("77\n"), expected, "{output}");
        }
    }
    #[test]
    fn native_files_round_trip_and_failed_loads_preserve_ram() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("r8-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("My Program.bas");
        let filename = path.to_string_lossy();
        let (_, output) = session(&format!(
            "20 PRINT A\n10 A=7\nSAVE \"{filename}\"\nNEW\nLOAD \"{filename}\"\nRUN\nQUIT\n"
        ));
        assert!(output.contains("> 7\n"), "{output}");
        let saved = std::fs::read_to_string(&path).unwrap();
        assert_eq!(saved, "10 A=7\n20 PRINT A\n");
        let mut reference = crate::basic::Basic::default();
        reference.load(&saved).unwrap();
        let mut reference_output = Vec::new();
        reference
            .run(&mut b"".as_slice(), &mut reference_output)
            .unwrap();
        assert_eq!(reference_output, b"7\n");
        for malformed in ["10 PRINT 8\nnot numbered", "0 END", "65536 END", "10 \0"] {
            std::fs::write(&path, malformed).unwrap();
            let (sys, failure_output) = session(&format!(
                "10 PRINT 7\nA=42\nLOAD \"{filename}\"\nPRINT A\nRUN\nQUIT\n"
            ));
            assert!(
                failure_output.contains("? ")
                    && failure_output.contains("> 42\n")
                    && failure_output.contains("> 7\n"),
                "{failure_output}"
            );
            assert_eq!(sys.mem.get(0x1002), b'P');
        }
        let mut too_many_lines = String::new();
        for line in 1..=257_u16 {
            use core::fmt::Write as _;
            writeln!(too_many_lines, "{line} REM").unwrap();
        }
        for invalid in [format!("10 REM {}", "x".repeat(200)), too_many_lines] {
            std::fs::write(&path, invalid).unwrap();
            let (_, rejected) = session(&format!("10 PRINT 7\nLOAD \"{filename}\"\nRUN\nQUIT\n"));
            assert!(
                rejected.contains("? ERROR") && rejected.contains("> 7\n"),
                "{rejected}"
            );
        }
        std::fs::remove_file(&path).unwrap();
        let (_, missing_output) = session(&format!("10 PRINT 7\nLOAD \"{filename}\"\nRUN\nQUIT\n"));
        assert!(
            missing_output.contains("? FILE ERROR") && missing_output.contains("> 7\n"),
            "{missing_output}"
        );
        std::fs::write(&path, "20 PRINT 2\r\n10 PRINT 1").unwrap();
        let (_, empty_output) = session(&format!(
            "LOAD \"{filename}\"\nRUN\nNEW\nSAVE \"{filename}\"\n10 PRINT 9\nLOAD \"{filename}\"\nRUN\nQUIT\n"
        ));
        assert!(
            empty_output.contains("1\n2\n") && !empty_output.contains("9\n"),
            "{empty_output}"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
