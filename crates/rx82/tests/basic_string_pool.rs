//! Pool capacity and string arrays on both interpreters and the real guest bus.
#![cfg(test)]
#![allow(clippy::unwrap_used, reason = "test assertions")]
use rx82::{basic::Basic, native, system::System};

fn session(commands: &str) -> (System, [String; 2]) {
    let mut basic = Basic::default();
    let mut output = Vec::new();
    basic
        .interact(&mut commands.as_bytes(), &mut output)
        .unwrap();
    let (mut sys, console) = native::machine();
    console.borrow_mut().input.extend(commands.bytes());
    console.borrow_mut().eof = true;
    for _ in 0..80_000_000_u32 {
        if sys.cpu.halt {
            break;
        }
        sys.tick();
    }
    assert!(sys.cpu.halt, "guest timed out at {:04X}", sys.cpu.pc);
    let native = String::from_utf8(console.borrow().output.clone()).unwrap();
    (sys, [String::from_utf8(output).unwrap(), native])
}

fn word(sys: &System, address: u16) -> u16 {
    u16::from_le_bytes([sys.mem.get(address), sys.mem.get(address.strict_add(1))])
}

#[test]
fn room_fixture_reclaims_replaced_text_and_rewrites_array_offsets() {
    let (mut sys, outputs) = session(&format!(
        "{}\nRUN\nQUIT\n",
        include_str!("../examples/rooms.bas")
    ));
    for output in outputs {
        assert!(output.contains("Entrance\tGarden\n"), "{output}");
        assert!(!output.contains('?'), "{output}");
    }
    // After compaction only Garden (7 bytes) and Entrance (9 bytes) are live.
    assert_eq!(word(&sys, 0x00B2), 0x8010);
    assert_eq!(word(&sys, 0x00B6), 0x9000);
    assert_eq!(word(&sys, 0x9000), 8);
    assert_eq!(word(&sys, 0x9002), 1);
    assert_eq!(sys.peek_mem(0x8000), 6);
    assert_eq!(sys.peek_mem(0x8007), 8);
}

#[test]
fn long_lengths_and_failed_assignment_preserve_old_value() {
    let full = "x".repeat(255);
    let (mut sys, outputs) = session(&format!(
        "INPUT A$\n{full}\nA$=A$+\"y\"\nPRINT LEN(A$)\nPRINT A$\nQUIT\n"
    ));
    for output in outputs {
        assert!(output.contains("? STRING TOO LONG"), "{output}");
        assert!(output.contains("> 255\n"), "{output}");
        assert!(output.contains(&full), "{output}");
    }
    assert_eq!(word(&sys, 0x0340), 1);
    assert_eq!(sys.peek_mem(0x8000), 255);
    assert_eq!(sys.peek_mem(0x80FF), b'x');
}

#[test]
fn pool_exhaustion_keeps_scalar_and_array_values_and_recovers() {
    let full = "q".repeat(255);
    let source = format!(
        "10 DIM R$(20)\n20 INPUT S$\n30 FOR I=0 TO 20\n40 R$(I)=S$\n50 NEXT I\nRUN\n{full}\nR$(0)=S$\nPRINT I,LEN(S$),LEN(R$(0))\nS$=\"\"\nR$(0)=\"ok\"\nPRINT R$(0),LEN(R$(1))\nNEW\nPRINT LEN(S$)\nQUIT\n"
    );
    let (sys, outputs) = session(&source);
    for output in outputs {
        assert_eq!(output.matches("? STRING SPACE").count(), 2, "{output}");
        assert!(output.contains("13\t255\t255\n"), "{output}");
        assert!(output.contains("ok\t255\n"), "{output}");
        assert!(output.contains("> 0\n"), "{output}");
    }
    assert_eq!(word(&sys, 0x00B2), 0x8000);
    assert_eq!(word(&sys, 0x00B6), 0x9000);
}

#[test]
fn scalar_names_array_types_read_input_and_len_remain_independent() {
    let (sys, outputs) = session(
        "DIM A(1)\nDIM A$(2)\nA=7\nA$=\"scalar\"\nA(0)=7\n100 DATA Hall,Garden\nREAD A$(0),A$(1)\nINPUT A$(2)\nRoof\nPRINT A,A(0),A$,LEN(A),LEN(A$),LEN((A$)),LEN(A$(1))\nA$(0)=\"Entry\"\nIF A$(0)+\"!\"=\"Entry!\" THEN PRINT A$(0),A$(1),A$(2)\nDIM A$(1)\nA$(3)=\"bad\"\nPRINT A$(0)\nQUIT\n",
    );
    for output in outputs {
        assert!(output.contains("7\t7\tscalar\t2\t3\t6\t6\n"), "{output}");
        assert!(output.contains("Entry\tGarden\tRoof\n"), "{output}");
        assert!(
            output
                .to_ascii_lowercase()
                .contains("array already dimensioned"),
            "{output}"
        );
        assert!(
            output
                .to_ascii_lowercase()
                .contains("subscript out of range"),
            "{output}"
        );
        assert!(output.contains("> Entry\n"), "{output}");
    }
    assert_eq!(word(&sys, 0x9000), 7);
    assert_eq!(word(&sys, 0x0300), 7);
}

#[test]
fn multidimensional_arrays_use_row_major_indices_and_survive_string_compaction() {
    let (sys, outputs) = session(
        "DIM A(1,2,3)\nDIM I(1)\nI(0)=1\nI(1)=2\nA(I(0),I(1),3)=73\nDIM S$(1,1)\nS$(0,0)=\"first\"\nS$(1,1)=\"old\"\nS$(1,1)=\"new\"\nPRINT LEN(A),A(1,2,3),A(0,0,0),LEN(S$),S$(0,0),S$(1,1)\nA(1,3,0)=1\nA(1,2)=1\nA(1,2,3,0)=1\nPRINT A(1,2,3)\nQUIT\n",
    );
    for output in outputs {
        assert!(output.contains("24\t73\t0\t4\tfirst\tnew\n"), "{output}");
        assert_eq!(output.matches("? ").count(), 3, "{output}");
        assert!(output.contains("> 73\n"), "{output}");
    }
    assert_eq!(word(&sys, 0x0800), 0x9006);
    assert_eq!(word(&sys, 0x0802), 0x1017);
}

#[test]
fn fragmented_read_reuses_free_blocks_and_arrays_share_capacity() {
    let (_, outputs) = session(
        "DIM R$(1)\n100 DATA aaaaaa,bbbb,xx,z\nREAD R$(0),R$(1),R$(0),R$(1)\nPRINT R$(0),R$(1)\nNEW\nDIM R$(2047)\nDIM B(0)\nQUIT\n",
    );
    for output in outputs {
        assert!(output.contains("xx\tz\n"), "{output}");
        assert!(
            output.to_ascii_lowercase().contains("array memory full"),
            "{output}"
        );
    }
}

#[test]
fn repeated_concatenation_and_nested_len_release_temporaries() {
    let (_, outputs) = session(
        "10 DIM A$(1)\n20 FOR I=1 TO 100\n30 A$(LEN(\"x\")-1)=A$(0)+(\"x\"+\"\")\n40 NEXT I\n50 PRINT LEN(A$(0)),LEN((A$(0)+A$(0)))\nRUN\nRUN\nQUIT\n",
    );
    for output in outputs {
        assert_eq!(output.matches("100\t200\n").count(), 2, "{output}");
        assert!(!output.contains('?'), "{output}");
    }
}
