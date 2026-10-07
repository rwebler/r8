//! Exercise strings and LEN on the reference interpreter and the R8 CPU.
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
    let native_output = String::from_utf8(shared.borrow().output.clone()).unwrap();
    (sys, [String::from_utf8(output).unwrap(), native_output])
}

fn stored_string(sys: &mut System, name: u8) -> Vec<u8> {
    let slot = 0x0340_u16.strict_add(u16::from(name.strict_sub(b'A')).strict_mul(2));
    let offset = u16::from_le_bytes([sys.mem.get(slot), sys.mem.get(slot.strict_add(1))]);
    if offset == 0 {
        return Vec::new();
    }
    let address = 0xEEFF_u16.strict_add(offset);
    let length = sys.peek_mem(address);
    (1..=u16::from(length))
        .map(|index| sys.peek_mem(address.strict_add(index)))
        .collect()
}

#[test]
fn strings_and_len_work_in_integer_expressions_and_guest_ram() {
    let (mut sys, outputs) = session(
        "10 A=7\n20 DIM A(LEN(\"abc\"))\n30 a$=\"MiXeD\"\n40 B$=(A$+(\" \"+\"case\"))\n50 A(LEN(A)-1)=2*LEN(B$)+LEN(A$)/5\n60 PRINT A,A(3),A$,B$,LEN(A),LEN(\"\"),LEN(Z$)\n70 FOR I=0 TO LEN(A)-1\n80 PRINT LEN((A$+\"!\"));\n90 NEXT I\n100 PRINT\nRUN\nQUIT\n",
    );
    for output in outputs {
        assert!(
            output.contains("7\t21\tMiXeD\tMiXeD case\t4\t0\t0\n6666\n"),
            "{output}"
        );
        assert!(!output.contains("? "), "{output}");
    }
    assert_eq!(stored_string(&mut sys, b'A'), b"MiXeD");
    assert_eq!(sys.mem.get(0x9006), 21);
    assert_eq!(sys.mem.get(0x0802), 3);
    assert_eq!(sys.mem.get(0x0300), 7);
}

#[test]
fn string_input_preserves_case_spaces_quotes_and_punctuation() {
    let input = "  MiXeD, \"quote\" +\t  ";
    let (mut sys, outputs) = session(&format!(
        "INPUT Z$\n{input}\nINPUT B$\n\nPRINT \"[\"+Z$+\"]\",LEN(Z$),\"[\"+B$+\"]\",LEN(B$)\nQUIT\n"
    ));
    for output in outputs {
        assert!(
            output.contains(&format!("[{input}]\t{}\t[]\t0\n", input.len())),
            "{output}"
        );
    }
    assert_eq!(stored_string(&mut sys, b'Z'), input.as_bytes());
}

#[test]
fn strings_compare_lexically_and_case_sensitively() {
    for (comparison, matches) in [
        ("\"\"=\"\"", true),
        ("\"a\"=\"A\"", false),
        ("\"a\"<>\"b\"", true),
        ("\"ab\"<>\"ab\"", false),
        ("\"a\"<\"ab\"", true),
        ("\"b\"<\"a\"", false),
        ("\"ab\"<=\"ab\"", true),
        ("\"ab\"<=\"a\"", false),
        ("\"z\">\"Z\"", true),
        ("\"\">\"a\"", false),
        ("\"abc\">=\"ab\"", true),
        ("\"A\">=\"a\"", false),
        ("(\"a\"+\"b\")=(\"ab\")", true),
    ] {
        let (_, outputs) = session(&format!("IF {comparison} THEN PRINT \"hit\"\nQUIT\n"));
        for output in outputs {
            assert_eq!(output.contains("hit\n"), matches, "{comparison}: {output}");
            assert!(!output.contains("? "), "{output}");
        }
    }
    let (_, outputs) =
        session("IF \"a\"=\"a\" THEN IF LEN(\"xy\")=2 THEN PRINT \"nested\"\nQUIT\n");
    for output in outputs {
        assert!(output.contains("nested\n"), "{output}");
    }
}

#[test]
fn failed_string_assignments_leave_existing_values_intact() {
    let full = "x".repeat(255);
    for (statement, cause) in [
        ("A$=A$+\"x\"".to_owned(), "string too long"),
        (format!("INPUT A$\n{}", "y".repeat(256)), "string too long"),
        ("A$=7".to_owned(), "type mismatch"),
        ("A$=\"ok\"+1".to_owned(), "type mismatch"),
        ("A$=\"\u{e9}\"".to_owned(), "invalid string character"),
        ("A$=(\"oops\"".to_owned(), "expected )"),
    ] {
        let (mut sys, outputs) = session(&format!(
            "INPUT A$\n{full}\nB$=\"safe\"\n{statement}\nPRINT LEN(A$),B$\nQUIT\n"
        ));
        for output in outputs {
            assert!(output.to_ascii_lowercase().contains(cause), "{output}");
            assert!(output.contains("255\tsafe\n"), "{output}");
        }
        assert_eq!(stored_string(&mut sys, b'A'), full.as_bytes());
    }
}

#[test]
fn len_rejects_wrong_types_and_reports_program_line() {
    for (statement, cause) in [
        ("PRINT LEN(1)", "type mismatch"),
        ("PRINT LEN(A(0))", "type mismatch"),
        ("PRINT LEN(Z)", "array not dimensioned"),
        ("PRINT LEN(\"x\"", "expected )"),
        ("PRINT LEN \"x\"", "expected ("),
        ("A=\"x\"", "type mismatch"),
        ("A(0)=\"x\"", "type mismatch"),
        ("IF 1=\"x\" THEN END", "type mismatch"),
        ("IF \"x\"=1 THEN END", "type mismatch"),
    ] {
        let (_, outputs) = session(&format!(
            "10 DIM A(0)\n20 {statement}\nRUN\nPRINT 99\nQUIT\n"
        ));
        for output in outputs {
            let lower = output.to_ascii_lowercase();
            assert!(
                lower.contains(cause) && lower.contains("line 20"),
                "{output}"
            );
            assert!(output.contains("> 99\n"), "{output}");
        }
    }
}

#[test]
fn strings_reset_on_run_and_new_without_reducing_array_capacity() {
    let (_, outputs) = session(
        "10 PRINT LEN(A$)\n20 A$=\"hello\"\nRUN\nRUN\nNEW\nDIM A(2047)\nZ$=\"last\"\nA$=\"\"\nA(2047)=LEN(Z$)\nPRINT LEN(A$),LEN(A),A(2047),Z$\nQUIT\n",
    );
    for output in outputs {
        assert_eq!(output.matches("> 0\n").count(), 2, "{output}");
        assert!(output.contains("0\t2048\t4\tlast\n"), "{output}");
        assert!(!output.contains("? "), "{output}");
    }
}

#[test]
fn all_string_scalars_coexist_with_a_full_array_pool() {
    let assignments = (b'A'..=b'Z')
        .map(|name| {
            let letter = char::from(name);
            format!("{letter}$=\"{}\"", letter.to_string().repeat(63))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let (mut sys, outputs) = session(&format!(
        "DIM A(2047)\nA(2047)=123\n{assignments}\nZ$=Z$+\"\"\nPRINT LEN(A),A(2047),LEN(A$),LEN(Z$)\nQUIT\n"
    ));
    for output in outputs {
        assert!(output.contains("2048\t123\t63\t63\n"), "{output}");
        assert!(!output.contains("? "), "{output}");
    }
    for name in b'A'..=b'Z' {
        assert_eq!(stored_string(&mut sys, name), vec![name; 63]);
    }
}
