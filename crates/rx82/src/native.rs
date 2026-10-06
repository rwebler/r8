//! Native BASIC ROM runner. The host transports bytes; the R8 interprets BASIC.
extern crate alloc;
use crate::{
    console::{Console, ConsoleState},
    system::System,
};
use alloc::rc::Rc;
use anyhow::Result;
use core::cell::RefCell;
use std::io::{BufRead, Write};

/// Native interpreter assembled from `sys/basic_rom.asm`.
pub const ROM: &[u8] = include_bytes!("../sys/basic_rom.bin");
/// Entry address of the optional BASIC extension ROM.
pub const ROM_START: u16 = 0xD000;

/// Creates a machine paused at the BASIC extension entry point.
///
/// # Panics
/// If the bundled module no longer fits in the firmware expansion window.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "bundled ROM layout is verified by tests"
)]
pub fn machine() -> (System, Rc<RefCell<ConsoleState>>) {
    let console = Console::default();
    let shared = Rc::clone(&console.shared);
    let mut sys = System {
        turbo: true,
        ..System::default()
    };
    sys.devices.insert(0, Box::new(console));
    sys.devices
        .insert(1, Box::<crate::files::FilePort>::default());
    sys.install_rom(ROM_START, ROM)
        .expect("valid BASIC ROM module");
    sys.enter_rom(ROM_START)
        .expect("installed BASIC ROM module");
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

/// Prepares a native debugging session, optionally loading before the first stop.
/// The source is always consumed by guest ROM instructions, never copied into records.
/// # Errors
/// Returns an error if preloading does not reach an input boundary within its budget.
pub fn debug_monitor(source: &str, break_before_run: bool) -> Result<crate::monitor::Monitor> {
    if break_before_run {
        anyhow::ensure!(
            source.lines().all(|line| {
                let line = line.trim();
                line.is_empty() || line.as_bytes().first().is_some_and(u8::is_ascii_digit)
            }),
            "pause-before-RUN requires a file of numbered BASIC lines"
        );
    }
    let (mut sys, shared) = machine();
    shared
        .borrow_mut()
        .input
        .extend(source.bytes().chain(*b"\n"));
    if break_before_run {
        let mut ready = false;
        for _ in 0..20_000_000_u32 {
            sys.tick();
            if sys.cpu.state == crate::state::State::FetchOpcode && shared.borrow().waiting {
                ready = true;
                break;
            }
            anyhow::ensure!(!sys.cpu.halt, "guest halted before loading completed");
        }
        anyhow::ensure!(
            ready,
            "guest did not finish loading within 20 million cycles"
        );
    }
    if break_before_run {
        anyhow::ensure!(
            !shared
                .borrow()
                .output
                .windows(2)
                .any(|bytes| bytes == b"? "),
            "the BASIC ROM rejected the source while loading: {}",
            String::from_utf8_lossy(&shared.borrow().output)
        );
    }
    shared.borrow_mut().input.extend(b"RUN\n");
    shared.borrow_mut().waiting = false;
    Ok(crate::monitor::Monitor {
        console: Some(shared),
        sys,
        ..Default::default()
    })
}

/// Opens the monitor at boot or after loading, with RUN queued for continuation.
/// # Errors
/// Returns guest preparation or terminal I/O errors.
pub fn debug(source: &str, break_before_run: bool) -> Result<()> {
    let mut monitor = debug_monitor(source, break_before_run)?;
    if break_before_run {
        println!("BASIC source loaded; paused before RUN. Use M 1000, then G.");
    }
    monitor.interact_at_current_pc()
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "tests")]
mod tests {
    use super::*;
    fn session(source: &str) -> (System, String) {
        session_with_budget(source, 5_000_000)
    }
    fn session_with_budget(source: &str, budget: u32) -> (System, String) {
        let (mut sys, shared) = machine();
        shared.borrow_mut().input.extend(source.bytes());
        shared.borrow_mut().eof = true;
        for _ in 0..budget {
            if sys.cpu.halt {
                break;
            }
            sys.tick();
        }
        assert!(
            sys.cpu.halt,
            "guest did not halt at {:04X} for {source}",
            sys.cpu.pc
        );
        let output = String::from_utf8(shared.borrow().output.clone()).unwrap();
        (sys, output)
    }
    #[test]
    fn native_division_matches_checked_signed_results() {
        let values = [
            i16::MIN,
            -30000,
            -257,
            -256,
            -181,
            -7,
            -3,
            -2,
            -1,
            0,
            1,
            2,
            3,
            7,
            127,
            181,
            255,
            256,
            257,
            30000,
            i16::MAX,
        ];
        for left in values {
            for right in values {
                let (result, cycles) = divide_routine(left, right, false);
                let expected = if right == 0 {
                    Err("division by zero")
                } else {
                    left.checked_div(right).ok_or("integer overflow")
                };
                assert_eq!(result, expected, "{left} / {right}");
                assert!(cycles < 3000, "{left} / {right} took {cycles} cycles");
            }
        }
    }

    #[test]
    fn native_division_cycle_regression() {
        for (left, right) in [(30000, 1), (i16::MIN, 1), (30000, 7), (i16::MIN, -1)] {
            let (old_result, old_cycles) = divide_routine(left, right, true);
            let (new_result, new_cycles) = divide_routine(left, right, false);
            assert_eq!(new_result, old_result);
            assert!(
                new_cycles.strict_mul(4) < old_cycles,
                "{left} / {right}: {old_cycles} -> {new_cycles} cycles"
            );
            println!("{left} / {right}: {old_cycles} -> {new_cycles} R8 cycles");
        }
    }

    fn divide_routine(left: i16, right: i16, legacy: bool) -> (Result<i16, &'static str>, u32) {
        use r8cpu::regs::Reg::{AB, CD, GH, SP};
        let (_, arithmetic) = include_str!("../sys/basic_rom.asm")
            .split_once("NEGATE:\n")
            .unwrap();
        let (helpers, _) = arithmetic.split_once("MUL_SIGNED:\n").unwrap();
        let (_, division) = arithmetic.split_once("DIV_SIGNED:\n").unwrap();
        let (division, _) = division.split_once("OVERFLOW:\n").unwrap();
        // Preserve the old algorithm as an isolated cycle-count baseline.
        let division = if legacy {
            "    push cd\n    cmp ef, 0x0000\n    bne LEGACY_DIVIDE\n    jmp DIV_ZERO\nLEGACY_DIVIDE:\n    call MAGNITUDES\n    ld cd, 0x0000\nDIV_LOOP:\n    cmp ab, ef\n    bcs LEGACY_SUBTRACT\n    jmp DIV_DONE\nLEGACY_SUBTRACT:\n    sec\n    sub ab, ef\n    inc cd\n    jmp DIV_LOOP\nDIV_DONE:\n    ld ab, cd\n    call MAG_RESULT\n    pop cd\n    ret\n"
        } else {
            division
        };
        let code = r8asm::assemble(&format!(
            "ld sp, 0xBFFF\nld ab, 0x{:04X}\nld ef, 0x{:04X}\nld cd, 0x1357\nld gh, 0x2468\ncall DIV_SIGNED\nhalt\nNEGATE:\n{helpers}\nDIV_SIGNED:\n{division}\nOVERFLOW:\nld gh, 0xFFFF\nhalt\nDIV_ZERO:\nld gh, 0xFFFE\nhalt",
            u16::from_le_bytes(left.to_le_bytes()), u16::from_le_bytes(right.to_le_bytes())
        )).unwrap();
        let mut sys = System {
            turbo: true,
            ..System::default()
        };
        sys.mem.load(0x0100, &code).unwrap();
        sys.cpu.pc = 0x0100;
        let mut cycles = 0_u32;
        while !sys.cpu.halt && cycles < 2_000_000 {
            sys.tick();
            cycles = cycles.strict_add(1);
        }
        assert!(sys.cpu.halt, "division timed out");
        match sys.cpu.regs.get16(GH) {
            0xFFFF => return (Err("integer overflow"), cycles),
            0xFFFE => return (Err("division by zero"), cycles),
            _ => {}
        }
        assert_eq!(sys.cpu.regs.get16(CD), 0x1357, "CD was not preserved");
        assert_eq!(
            sys.cpu.regs.get16(GH),
            0x2468,
            "source pointer was not preserved"
        );
        assert_eq!(sys.cpu.regs.get16(SP), 0xBFFF, "unbalanced stack");
        (
            Ok(i16::from_le_bytes(sys.cpu.regs.get16(AB).to_le_bytes())),
            cycles,
        )
    }

    #[test]
    fn native_multiplication_matches_checked_signed_results() {
        let values = [
            i16::MIN,
            -30000,
            -256,
            -255,
            -181,
            -2,
            -1,
            0,
            1,
            2,
            3,
            127,
            181,
            255,
            256,
            30000,
            i16::MAX,
        ];
        for left in values {
            for right in values {
                let (result, cycles) = multiply_routine(left, right, false);
                assert_eq!(result, left.checked_mul(right), "{left} * {right}");
                assert!(cycles < 2500, "{left} * {right} took {cycles} cycles");
            }
        }
    }

    #[test]
    fn native_multiplication_cycle_regression() {
        for (left, right) in [(30000, 1), (i16::MIN, 1), (30000, 0), (181, 181)] {
            let (old_result, old_cycles) = multiply_routine(left, right, true);
            let (new_result, new_cycles) = multiply_routine(left, right, false);
            assert_eq!(new_result, old_result);
            assert!(
                new_cycles.strict_mul(4) < old_cycles,
                "{left} * {right}: {old_cycles} -> {new_cycles} cycles"
            );
            println!("{left} * {right}: {old_cycles} -> {new_cycles} R8 cycles");
        }
    }

    // Run the real arithmetic routine in isolation, including its sign and
    // overflow helpers. The legacy body is a benchmark fixture, not guest code.
    fn multiply_routine(left: i16, right: i16, legacy: bool) -> (Option<i16>, u32) {
        use r8cpu::regs::Reg::{AB, CD, GH, SP};
        let (_, arithmetic) = include_str!("../sys/basic_rom.asm")
            .split_once("NEGATE:\n")
            .unwrap();
        let (arithmetic, _) = arithmetic.split_once("DIV_SIGNED:\n").unwrap();
        let body = if legacy {
            let (helpers, _) = arithmetic.split_once("MUL_SIGNED:\n").unwrap();
            format!(
                "{helpers}MUL_SIGNED:\n    push cd\n    call MAGNITUDES\n    ld cd, 0x0000\nMUL_LOOP:\n    cmp ab, 0x0000\n    bne LEGACY_ADD\n    jmp MUL_DONE\nLEGACY_ADD:\n    clc\n    add cd, ef\n    bcc LEGACY_NEXT\n    jmp OVERFLOW\nLEGACY_NEXT:\n    dec ab\n    jmp MUL_LOOP\nMUL_DONE:\n    ld ab, cd\n    call MAG_RESULT\n    pop cd\n    ret\n"
            )
        } else {
            arithmetic.to_owned()
        };
        let code = r8asm::assemble(&format!(
            "ld sp, 0xBFFF\nld ab, 0x{:04X}\nld ef, 0x{:04X}\nld cd, 0x1357\nld gh, 0x2468\ncall MUL_SIGNED\nhalt\nNEGATE:\n{body}\nOVERFLOW:\nld gh, 0xFFFF\nhalt",
            u16::from_le_bytes(left.to_le_bytes()), u16::from_le_bytes(right.to_le_bytes())
        )).unwrap();
        let mut sys = System {
            turbo: true,
            ..System::default()
        };
        sys.mem.load(0x0100, &code).unwrap();
        sys.cpu.pc = 0x0100;
        let mut cycles = 0_u32;
        while !sys.cpu.halt && cycles < 2_000_000 {
            sys.tick();
            cycles = cycles.strict_add(1);
        }
        assert!(sys.cpu.halt, "multiplication timed out");
        if sys.cpu.regs.get16(GH) == 0xFFFF {
            return (None, cycles);
        }
        assert_eq!(sys.cpu.regs.get16(CD), 0x1357, "CD was not preserved");
        assert_eq!(
            sys.cpu.regs.get16(GH),
            0x2468,
            "source pointer was not preserved"
        );
        assert_eq!(sys.cpu.regs.get16(SP), 0xBFFF, "unbalanced stack");
        (
            Some(i16::from_le_bytes(sys.cpu.regs.get16(AB).to_le_bytes())),
            cycles,
        )
    }

    #[test]
    fn native_peek_and_poke_access_variables_arrays_strings_and_source() {
        let (sys, output) = session(
            "10 PRINT 1\nA=4660\nDIM B(1)\nB(1)=1027\nC$=\"cat\"\nPRINT PEEK(768),PEEK(769),PEEK(36866),PEEK(4096),PEEK(4098)\nPOKE 768,120\nPOKE 36866,255\nPOKE 2432,98\nPOKE 4104,50\nPRINT A,B(1),C$\nRUN\nQUIT\n",
        );
        assert!(output.contains("52\t18\t3\t10\t80\n"), "{output}");
        assert!(output.contains("4728\t1279\tbat\n"), "{output}");
        assert!(output.contains("> 2\n"), "{output}");
        assert_eq!(sys.mem.get(4104), b'2');
    }

    #[test]
    fn native_memory_access_uses_devices_and_keeps_rom_read_only() {
        let (_, output) = session(
            "A=PEEK(53248)\nPOKE 53248,255\nPRINT A,PEEK(53248)\nPOKE 65535,0\nPRINT PEEK(65535),PEEK(-1)\nPOKE 65282,126\nPRINT PEEK(65281),PEEK(65281)\nABQUIT\n",
        );
        let first_byte = ROM.first().unwrap();
        assert!(
            output.contains(&format!("{first_byte}\t{first_byte}\n")),
            "{output}"
        );
        assert!(output.contains("192\t192\n"), "{output}");
        assert_eq!(output.matches('~').count(), 1, "{output}");
        assert!(output.contains("65\t66\n"), "{output}");
    }

    #[test]
    fn native_invalid_memory_operations_do_not_write_or_consume_input() {
        for statement in [
            "POKE 256,-1",
            "POKE 256,256",
            "POKE 256,1 2",
            "POKE 256 1",
            "POKE 65536,1",
            "PRINT PEEK(65281",
            "PRINT PEEK 256",
        ] {
            let (sys, output) = session(&format!(
                "POKE 256,42\n{statement}\nPRINT PEEK(256)\nQUIT\n"
            ));
            assert!(output.contains("? "), "{statement}: {output}");
            assert!(output.contains("> 42\n"), "{statement}: {output}");
            assert_eq!(sys.mem.get(256), 42);
        }
    }

    #[test]
    fn native_arrays_live_in_ram_and_support_nested_indices() {
        let (sys, output) = session(
            "DIM A(100)\nA=7\nA(0)=100\nINPUT A(A(0))\n-32768\nLET A(1)=A(100)+2\nPRINT A,A(2),A(100),A(1)\nQUIT\n",
        );
        assert!(output.contains("7\t0\t-32768\t-32766\n"), "{output}");
        for (address, expected) in [
            (0x0800, 0x9000),
            (0x0802, 100),
            (0x9000, 100),
            (0x9002, 0x8002),
            (0x90C8, 0x8000),
            (0x0300, 7),
        ] {
            assert_eq!(
                u16::from_le_bytes([sys.mem.get(address), sys.mem.get(address.strict_add(1))]),
                expected
            );
        }
    }

    #[test]
    fn native_array_errors_recover_without_consuming_storage() {
        let (_, output) = session(
            "PRINT A(0)\nA(0)=1\nDIM A(-1)\nDIM A(2048)\nDIM A(2046)\nDIM A(0)\nA(-1)=3\nPRINT A(2047)\nDIM B(1)\nDIM B(0)\nB(0)=42\nPRINT A(2046),B(0)\nQUIT\n",
        );
        for cause in [
            "ARRAY NOT DIMENSIONED",
            "SUBSCRIPT OUT OF RANGE",
            "ARRAY ALREADY DIMENSIONED",
            "ARRAY MEMORY FULL",
        ] {
            assert!(output.contains(cause), "{output}");
        }
        assert!(output.contains("0\t42\n"), "{output}");
    }

    #[test]
    fn native_arrays_reset_on_run_and_new() {
        let (_, output) = session(
            "10 DIM A(0)\n20 PRINT A(0)\n30 A(0)=42\nRUN\nRUN\nNEW\nPRINT A(0)\nDIM A(2047)\nA(2047)=123\nPRINT A(2047)\nQUIT\n",
        );
        assert_eq!(output.matches("> 0\n").count(), 2, "{output}");
        assert!(output.contains("ARRAY NOT DIMENSIONED"), "{output}");
        assert!(output.contains("> 123\n"), "{output}");
        assert!(!output.contains("ARRAY MEMORY FULL"), "{output}");
    }

    #[test]
    fn native_module_keeps_stock_firmware_and_reset_vector() {
        let (mut sys, _) = machine();
        for addr in 0xC000..0xD000 {
            assert_eq!(
                Some(sys.peek_mem(addr)),
                crate::system::ROM_DATA
                    .get(usize::from(addr.strict_sub(0xC000)))
                    .copied()
            );
        }
        assert_eq!(sys.peek_mem(0xFFFE), 0);
        assert_eq!(sys.peek_mem(0xFFFF), 0xC0);
        sys.cpu.reset(&mut sys.bus);
        for _ in 0_u8..32 {
            if sys.cpu.state == crate::state::State::FetchOpcode {
                break;
            }
            sys.tick();
        }
        assert_eq!(sys.cpu.state, crate::state::State::FetchOpcode);
        assert_eq!(sys.cpu.pc, 0xC000);
        sys.enter_rom(ROM_START).unwrap();
        assert_eq!(sys.cpu.pc, ROM_START);
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
        assert_eq!(monitor.sys.cpu.pc, ROM_START);
        monitor.sys.debug_print();
        monitor.memory(Some(0x1000));
        assert_eq!(monitor.sys.cpu.pc, ROM_START);
        monitor.step(None).unwrap();
        monitor.go(None).unwrap();
        assert_eq!(monitor.sys.mem.get(0x0300), 42);
        assert_eq!(monitor.sys.mem.get(0x1000), 10);
    }

    #[test]
    fn debugger_pauses_with_source_loaded_before_any_program_statement() {
        use crate::monitor::StopReason;
        let mut monitor = debug_monitor("10 A=42\n20 PRINT A\n30 END", true).unwrap();
        assert_eq!(monitor.sys.mem.get(0x1000), 10);
        assert_eq!(monitor.sys.mem.get(0x1002), b'A');
        assert_eq!(monitor.sys.mem.get(0x0300), 0);
        assert_eq!(monitor.sys.mem.get(0x0082), 0);
        let mut output = Vec::new();
        assert_eq!(
            monitor.run_with_output(None, &mut output).unwrap(),
            StopReason::WaitingForInput
        );
        assert_eq!(monitor.sys.mem.get(0x0300), 42);
        assert!(String::from_utf8(output).unwrap().contains("42\n"));
    }

    #[test]
    fn debugger_delivers_output_and_guest_input() {
        let mut monitor = debug_monitor("10 INPUT A\n20 PRINT A\n30 END", true).unwrap();
        let mut output = Vec::new();
        monitor
            .interact_with(&mut b"G\nI 7\nG\nI QUIT\nG\nQ\n".as_slice(), &mut output)
            .unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("Guest waiting for input"), "{text}");
        assert!(text.contains("7\n> Guest waiting"), "{text}");
        assert!(text.contains("Halted."), "{text}");
        assert_eq!(monitor.sys.mem.get(0x0300), 7);
    }

    #[test]
    fn preload_rejects_commands_and_invalid_numbered_source() {
        assert!(debug_monitor("10 A=42\nRUN", true).is_err());
        assert!(debug_monitor("0 PRINT 1", true).is_err());
    }

    #[test]
    fn rom_matches_source_and_fits() {
        assert_eq!(
            r8asm::assemble(include_str!("../sys/basic_rom.asm")).unwrap(),
            ROM
        );
        assert!(ROM.len() <= 0x2F00);
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
        assert!(sys.cpu.pc >= ROM_START);
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
            "10 POKE 256,1\n20 POKE 257,200\n30 P=-28672\n40 POKE P+1,129\n50 PRINT PEEK(PEEK(256)+256),2*PEEK(P+1)+1,PEEK(36865)",
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
    fn native_runtime_diagnostics_report_cause_and_line() {
        for (source, message, line) in [
            ("10 NEXT I", "NEXT WITHOUT FOR", 10_u16),
            ("10 GOTO 999", "UNDEFINED LINE", 10),
            ("10 GOSUB 999", "UNDEFINED LINE", 10),
            ("10 GOTO 65535", "UNDEFINED LINE", 10),
            ("65535 GOTO 1", "UNDEFINED LINE", u16::MAX),
            ("10 RETURN", "RETURN WITHOUT GOSUB", 10),
            ("10 FOR I=1 TO 2 STEP 0\n20 NEXT", "ZERO STEP", 10),
            ("10 FOR I=1 TO 2", "FOR WITHOUT NEXT", 10),
            ("10 FOR I=1 TO 2\n20 NEXT J", "NEXT MISMATCH", 10),
            (
                "10 FOR I=1 TO 2\n20 FOR I=1 TO 2\n30 NEXT\n40 NEXT",
                "FOR VARIABLE ALREADY ACTIVE",
                20,
            ),
            (
                "10 GOTO 30\n20 FOR I=1 TO 2\n30 NEXT I",
                "NEXT WITHOUT FOR",
                30,
            ),
            ("10 FOR I=32767 TO 32767\n20 NEXT I", "INTEGER OVERFLOW", 20),
            ("10 GOSUB 10", "GOSUB STACK FULL", 10),
            ("10 A 3", "EXPECTED =", 10),
            ("10 PRINT \"hello", "UNTERMINATED STRING", 10),
            ("10 PRINT (1+2", "EXPECTED )", 10),
            ("10 LET 1=2", "EXPECTED VARIABLE A-Z", 10),
            ("10 IF 1 THEN PRINT 2", "EXPECTED COMPARISON", 10),
            ("10 IF 1=1 PRINT 2", "EXPECTED THEN", 10),
            ("10 FOR I=1 2\n20 NEXT", "EXPECTED TO", 10),
            ("10 IF 1=1 THEN NEXT I", "FOR/NEXT MUST STAND ALONE", 10),
            ("10 END extra", "UNEXPECTED INPUT", 10),
            ("10 NEW", "DIRECT MODE ONLY", 10),
        ] {
            let (_, output) = session_with_budget(
                &format!("{source}\nRUN\nPRINT 7\nNEW\n10 PRINT 8\nRUN\nQUIT\n"),
                20_000_000,
            );
            assert!(
                output.contains(&format!("? {message} IN LINE {line}\n")),
                "{source}\n{output}"
            );
            assert_eq!(output.matches("? ").count(), 1, "{output}");
            assert!(
                output.contains("> 7\n") && output.contains("8\n"),
                "{output}"
            );
        }
    }

    #[test]
    fn native_direct_errors_have_no_stale_program_line() {
        for (command, message) in [
            ("0 PRINT 1", "INVALID LINE NUMBER"),
            ("GOTO 10", "REQUIRES RUN"),
            ("SAVE name", "EXPECTED QUOTED FILENAME"),
            ("SAVE \"name", "UNTERMINATED STRING"),
            ("PRINT 1 garbage", "UNEXPECTED INPUT"),
        ] {
            let (_, output) = session(&format!("10 END\nRUN\n{command}\nPRINT 7\nQUIT\n"));
            assert!(output.contains(&format!("? {message}\n")), "{output}");
            assert!(!output.contains(" IN LINE "), "{output}");
            assert!(output.contains("> 7\n"), "{output}");
        }
    }

    #[test]
    fn native_full_program_preserves_records_and_allows_editing() {
        use core::fmt::Write as _;
        let mut source = String::new();
        for line in 1..=256_u16 {
            writeln!(source, "{line} REM").unwrap();
        }
        source.push_str("257 REM extra\nPRINT 7\n1 PRINT 42\n256\n257 REM replacement\nQUIT\n");
        let (sys, output) = session_with_budget(&source, 20_000_000);
        assert!(output.contains("? PROGRAM FULL\n"), "{output}");
        assert_eq!(output.matches("? ").count(), 1, "{output}");
        assert!(output.contains("> 7\n"), "{output}");
        assert_eq!(sys.mem.get(0x1000), 1);
        assert_eq!(sys.mem.get(0x1002), b'P');
        assert_eq!(
            u16::from_le_bytes([sys.mem.get(0x8F80), sys.mem.get(0x8F81)]),
            257
        );
    }

    #[test]
    fn native_rejected_input_discards_the_rest_of_the_line() {
        for (line, message) in [
            (
                format!("10 REM {}PRINT 99", "x".repeat(130)),
                "LINE TOO LONG",
            ),
            ("10 REM \0PRINT 99".to_owned(), "INVALID CHARACTER"),
        ] {
            let (_, output) = session(&format!("{line}\nPRINT 7\nQUIT\n"));
            assert!(output.contains(&format!("? {message}\n")), "{output}");
            assert_eq!(output.matches("? ").count(), 1, "{output}");
            assert!(output.contains("> 7\n"), "{output}");
            let (_, eof_output) = session(&line);
            assert!(
                eof_output.contains(&format!("? {message}\n")),
                "{eof_output}"
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
        let (_, string_output) = session(&format!(
            "10 A$=\"Saved text\"\n20 PRINT A$\nSAVE \"{filename}\"\nNEW\nLOAD \"{filename}\"\nRUN\nQUIT\n"
        ));
        assert!(string_output.contains("> Saved text\n"), "{string_output}");
        let string_source = std::fs::read_to_string(&path).unwrap();
        assert_eq!(string_source, "10 A$=\"Saved text\"\n20 PRINT A$\n");
        reference.load(&string_source).unwrap();
        reference_output.clear();
        reference
            .run(&mut b"".as_slice(), &mut reference_output)
            .unwrap();
        assert_eq!(reference_output, b"Saved text\n");
        let (_, data_output) = session(&format!(
            "10 READ A$\n20 PRINT A$\n30 END\n100 DATA \"Saved, data\",7\nSAVE \"{filename}\"\nREAD N$\nLOAD \"{filename}\"\nREAD N$,N\nPRINT N$,N\nRUN\nQUIT\n"
        ));
        assert!(data_output.contains("Saved, data\t7\n"), "{data_output}");
        assert!(data_output.contains("> Saved, data\n"), "{data_output}");
        let data_source = std::fs::read_to_string(&path).unwrap();
        assert!(data_source.contains("100 DATA \"Saved, data\",7\n"));
        reference.load(&data_source).unwrap();
        reference_output.clear();
        reference
            .run(&mut b"".as_slice(), &mut reference_output)
            .unwrap();
        assert_eq!(reference_output, b"Saved, data\n");
        let (_, loaded) = session(&format!(
            "DIM A(0)\nA(0)=99\nA$=\"old\"\nLOAD \"{filename}\"\nPRINT LEN(A$)\nPRINT A(0)\nDIM A(2047)\nPRINT A(2047)\nQUIT\n"
        ));
        assert!(loaded.contains("ARRAY NOT DIMENSIONED"), "{loaded}");
        assert!(loaded.contains("> 0\n"), "{loaded}");
        assert_eq!(loaded.matches("> 0\n").count(), 2, "{loaded}");
        assert!(!loaded.contains("ARRAY MEMORY FULL"), "{loaded}");
        for malformed in ["10 PRINT 8\nnot numbered", "0 END", "65536 END", "10 \0"] {
            std::fs::write(&path, malformed).unwrap();
            let (sys, failure_output) = session(&format!(
                "10 PRINT 7\n100 DATA 5,6\nREAD D\nA=42\nDIM B(0)\nB(0)=99\nA$=\"kept\"\nLOAD \"{filename}\"\nREAD D\nPRINT D\nPRINT A\nPRINT B(0)\nPRINT A$\nRUN\nQUIT\n"
            ));
            assert!(
                failure_output.contains("? ")
                    && failure_output.contains("> 42\n")
                    && failure_output.contains("> 99\n")
                    && failure_output.contains("> kept\n")
                    && failure_output.contains("> 6\n")
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
        for (invalid, expected) in [
            (format!("10 REM {}", "x".repeat(200)), "LINE TOO LONG"),
            (too_many_lines, "PROGRAM FULL"),
        ] {
            std::fs::write(&path, invalid).unwrap();
            let (_, rejected) = session(&format!("10 PRINT 7\nLOAD \"{filename}\"\nRUN\nQUIT\n"));
            assert!(
                rejected.contains(&format!("? {expected}\n")) && rejected.contains("> 7\n"),
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
