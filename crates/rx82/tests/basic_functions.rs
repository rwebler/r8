//! User defined numeric functions shared by both BASIC runners.
#![cfg(test)]
#![allow(clippy::unwrap_used, reason = "test assertions")]
use rx82::{basic::Basic, native};

fn outputs(source: &str) -> [String; 2] {
    let mut reference = Basic::default();
    let mut host = Vec::new();
    reference
        .interact(&mut source.as_bytes(), &mut host)
        .unwrap();
    let (mut sys, console) = native::machine();
    console.borrow_mut().input.extend(source.bytes());
    console.borrow_mut().eof = true;
    for _ in 0..50_000_000_u32 {
        if sys.cpu.halt {
            break;
        }
        sys.tick();
    }
    assert!(sys.cpu.halt, "guest timed out at {:04X}", sys.cpu.pc);
    let guest = String::from_utf8(console.borrow().output.clone()).unwrap();
    [String::from_utf8(host).unwrap(), guest]
}

#[test]
fn forward_nested_functions_preserve_globals_and_list_spacing() {
    for output in outputs(
        "10 X=7\n20 PRINT FNA(3),X\n30 END\n40 DEF FNA(X)=FNB(X+1)+X\n50 DEF FNB(Y)=Y*2\nLIST\nRUN\nQUIT\n",
    ) {
        assert!(output.contains("40 DEF FNA(X)=FNB(X+1)+X"), "{output}");
        assert!(output.contains("11\t7\n"), "{output}");
    }
}

#[test]
fn definitions_rebuild_after_edits_and_errors_recover() {
    for output in outputs(
        "10 PRINT FNA(5)\n20 END\n30 DEF FNA(X)=X*X+1\nRUN\n30 DEF FNA(X)=X+2\nRUN\n30 DEF FNA(X)=X+32767\nRUN\n30 DEF FNA(X)=X+1\nRUN\nPRINT 9\nQUIT\n",
    ) {
        assert!(output.contains("26\n"), "{output}");
        assert!(output.contains("7\n"), "{output}");
        assert!(output.to_ascii_lowercase().contains("overflow"), "{output}");
        assert!(output.contains("6\n"), "{output}");
        assert!(output.contains("9\n"), "{output}");
    }
}
