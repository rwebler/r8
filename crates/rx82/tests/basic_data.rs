//! DATA cursor behavior on the reference interpreter and native R8 ROM.
#![expect(clippy::unwrap_used, reason = "test assertions")]
#![expect(clippy::tests_outside_test_module, reason = "integration test crate")]

use rx82::{basic::Basic, native, system::System};

fn session(commands: &str) -> (System, [String; 2]) {
    let mut reference = Basic::default();
    let mut output = Vec::new();
    reference
        .interact(&mut commands.as_bytes(), &mut output)
        .unwrap();
    let (mut sys, shared) = native::machine();
    shared.borrow_mut().input.extend(commands.bytes());
    shared.borrow_mut().eof = true;
    for _ in 0..20_000_000_u32 {
        if sys.cpu.halt {
            break;
        }
        sys.tick();
    }
    assert!(sys.cpu.halt, "guest timed out at {:04X}", sys.cpu.pc);
    let guest = String::from_utf8(shared.borrow().output.clone()).unwrap();
    (sys, [String::from_utf8(output).unwrap(), guest])
}

#[test]
fn read_fills_arrays_in_line_order_and_crosses_data_lines() {
    let (sys, outputs) = session(
        "200 DATA 3,4\n100 DATA -32768,+32767\n10 DIM A(3)\n20 FOR I=0 TO LEN(A)-1\n30 READ A(I)\n40 NEXT I\n50 READ N$,G$\n60 PRINT A(0),A(1),A(2),A(3),N$,G$\n70 END\n150 REM DATA 99 IS NOT A DATA STATEMENT\n300 data MiXeD,\"Hello, there\"\nRUN\nQUIT\n",
    );
    for output in outputs {
        assert!(
            output.contains("-32768\t32767\t3\t4\tMiXeD\tHello, there\n"),
            "{output}"
        );
        assert!(!output.contains("? "), "{output}");
    }
    for (index, value) in [i16::MIN, i16::MAX, 3, 4].into_iter().enumerate() {
        let address = 0x9000_u16.strict_add(u16::try_from(index).unwrap().strict_mul(2));
        assert_eq!(
            i16::from_le_bytes([sys.mem.get(address), sys.mem.get(address.strict_add(1))]),
            value
        );
    }
}

#[test]
fn read_targets_see_prior_assignments_and_share_the_subroutine_cursor() {
    let (_, outputs) = session(
        "10 DIM A(2)\n20 READ I,A(I),N$\n30 GOSUB 100\n40 PRINT I,A(2),N$,B\n50 END\n100 READ B\n110 RETURN\n200 DATA 2,42,world,99\nRUN\nQUIT\n",
    );
    for output in outputs {
        assert!(output.contains("2\t42\tworld\t99\n"), "{output}");
        assert!(!output.contains("? "), "{output}");
    }
}

#[test]
fn restore_rewinds_or_starts_at_an_existing_line() {
    let (_, outputs) = session(
        "100 DATA 1,2\n150 REM START SEARCHING HERE\n200 DATA 3\n65535 DATA 4\nREAD A,B,C,D\nPRINT A,B,C,D\nRESTORE 150\nREAD A\nPRINT A\nRESTORE 65535\nREAD A\nPRINT A\nRESTORE\nREAD A\nRESTORE 999\nREAD B\nPRINT A,B\nREAD C,D\nREAD E\nPRINT 99\nQUIT\n",
    );
    for output in outputs {
        assert!(output.contains("1\t2\t3\t4\n"), "{output}");
        assert!(
            output.contains("> 3\n") && output.contains("> 4\n"),
            "{output}"
        );
        assert!(output.contains("1\t2\n"), "{output}");
        let lower = output.to_ascii_lowercase();
        assert!(
            lower.contains("undefined line") && lower.contains("out of data"),
            "{output}"
        );
        assert!(output.contains("> 99\n"), "{output}");
    }
}

#[test]
fn data_strings_preserve_quoted_whitespace_and_unquoted_case() {
    let (_, outputs) = session(
        "100 DATA  MiXeD words  ,\" x,y:z \" ,\"\",123,Don't evaluate 1+2\nREAD A$,B$,C$,D$,E$\nPRINT \"[\"+A$+\"]\",\"[\"+B$+\"]\",LEN(C$),D$,E$\nQUIT\n",
    );
    for output in outputs {
        assert!(
            output.contains("[MiXeD words]\t[ x,y:z ]\t0\t123\tDon't evaluate 1+2\n"),
            "{output}"
        );
        assert!(!output.contains("? "), "{output}");
    }
}

#[test]
fn failed_reads_preserve_the_destination_and_current_item() {
    for (item, error, recovered) in [
        ("wrong", "type mismatch", "wrong"),
        ("\"12\"", "type mismatch", "12"),
        ("32768", "integer overflow", "32768"),
        ("-32769", "integer overflow", "-32769"),
        ("1+2", "type mismatch", "1+2"),
    ] {
        let (_, outputs) = session(&format!(
            "10 A=77\n20 READ A\n30 END\n100 DATA {item},9\nRUN\nPRINT A\nREAD N$,B\nPRINT N$,B\nQUIT\n"
        ));
        for output in outputs {
            let lower = output.to_ascii_lowercase();
            assert!(
                lower.contains(error) && lower.contains("line 20"),
                "{output}"
            );
            assert!(output.contains("> 77\n"), "{output}");
            assert!(output.contains(&format!("{recovered}\t9\n")), "{output}");
        }
    }
    let (_, outputs) =
        session("100 DATA 5,6\nDIM A(0)\nREAD A(1)\nREAD A(0),B\nPRINT A(0),B\nQUIT\n");
    for output in outputs {
        assert!(
            output
                .to_ascii_lowercase()
                .contains("subscript out of range"),
            "{output}"
        );
        assert!(output.contains("5\t6\n"), "{output}");
    }
}

#[test]
fn malformed_data_is_reported_when_read_and_unused_data_is_skipped() {
    for (item, error) in [
        ("", "invalid data"),
        (",1", "invalid data"),
        ("\"x\" junk", "invalid data"),
        ("a:b", "invalid data"),
        ("\"unfinished", "unterminated string"),
    ] {
        let (_, outputs) = session(&format!(
            "10 PRINT 7\n100 DATA {item}\nRUN\nREAD A$\nPRINT 8\nQUIT\n"
        ));
        for output in outputs {
            assert!(output.contains("> 7\n"), "{output}");
            assert!(output.to_ascii_lowercase().contains(error), "{output}");
            assert!(output.contains("> 8\n"), "{output}");
        }
    }
    let (_, outputs) = session("100 DATA 1,\nREAD A,B\nPRINT A\nQUIT\n");
    for output in outputs {
        assert!(
            output.to_ascii_lowercase().contains("invalid data"),
            "{output}"
        );
        assert!(output.contains("> 1\n"), "{output}");
    }
}

#[test]
fn run_new_and_program_edits_reset_the_data_cursor() {
    let (_, outputs) = session(
        "10 READ A\n20 PRINT A\n30 END\n100 DATA 7,8\nRUN\nRUN\nREAD B\nPRINT B\n100 DATA 9,10\nREAD B\nPRINT B\n100\nREAD B\nNEW\n100 DATA 11\nREAD B\nPRINT B\nQUIT\n",
    );
    for output in outputs {
        assert_eq!(output.matches("> 7\n").count(), 2, "{output}");
        for value in [8_u8, 9, 11] {
            assert!(output.contains(&format!("> {value}\n")), "{output}");
        }
        assert!(
            output.to_ascii_lowercase().contains("out of data"),
            "{output}"
        );
    }
}
