//! Execute the shipped BASIC programs on both interpreters and inspect guest RAM.
#![expect(clippy::unwrap_used, reason = "test assertions")]
#![expect(
    clippy::tests_outside_test_module,
    reason = "this entire integration test crate contains tests"
)]

use rx82::{basic::Basic, native, system::System};

fn check_example(source: &str, input: &str, expected: &str, variables: &[(u8, i16)]) {
    let mut reference = Basic::default();
    reference.load(source).unwrap();
    let mut reference_output = Vec::new();
    reference
        .run(&mut input.as_bytes(), &mut reference_output)
        .unwrap();
    assert_eq!(reference_output, expected.as_bytes(), "reference output");

    let (mut sys, shared) = native::machine();
    let commands = format!("{source}\nPRINT \"BEGIN\"\nRUN\n{input}PRINT \"FINISH\"\nQUIT\n");
    shared.borrow_mut().input.extend(commands.bytes());
    shared.borrow_mut().eof = true;
    for _ in 0..20_000_000_u32 {
        if sys.cpu.halt {
            break;
        }
        sys.tick();
    }
    assert!(sys.cpu.halt, "guest timed out at {:04X}", sys.cpu.pc);
    assert!(
        sys.cpu.pc >= native::ROM_START,
        "execution finished outside BASIC ROM"
    );
    let output = String::from_utf8(shared.borrow().output.clone()).unwrap();
    let (_, body) = output.split_once("BEGIN\n> ").unwrap();
    let (body, _) = body.split_once("> FINISH\n").unwrap();
    assert_eq!(body.as_bytes(), reference_output, "native output: {output}");
    inspect_program(&sys, source);
    for &(name, expected_value) in variables {
        let address = 0x0300_u16.strict_add(u16::from(name.strict_sub(b'A')).strict_mul(2));
        let actual = i16::from_le_bytes([sys.mem.get(address), sys.mem.get(address.strict_add(1))]);
        assert_eq!(
            actual,
            expected_value,
            "guest variable {}",
            char::from(name)
        );
    }
}

#[expect(
    clippy::single_call_fn,
    reason = "separate source RAM inspection from execution"
)]
fn inspect_program(sys: &System, source: &str) {
    let mut reference = Basic::default();
    reference.load(source).unwrap();
    for (offset, &byte) in reference.program_image().iter().enumerate() {
        let address = 0x1000_u16.strict_add(u16::try_from(offset).unwrap());
        assert_eq!(
            sys.mem.get(address),
            byte,
            "guest token chain at {address:04X}"
        );
    }
    assert_eq!(
        u16::from_le_bytes([sys.mem.get(0x86), sys.mem.get(0x87)]),
        reference.program_end(),
        "guest end marker address"
    );
}

#[test]
fn countdown() {
    check_example(
        include_str!("../examples/countdown.bas"),
        "",
        "10\n8\n6\n4\n2\n0\nLift off!\n",
        &[(b'I', -2)],
    );
}

#[test]
fn squares() {
    check_example(
        include_str!("../examples/squares.bas"),
        "5\n",
        "How many squares?\n? 1\t1\n2\t4\n3\t9\n4\t16\n5\t25\n",
        &[(b'I', 6), (b'N', 5)],
    );
}

#[test]
fn fibonacci() {
    check_example(
        include_str!("../examples/fibonacci.bas"),
        "",
        "N\tFIBONACCI\n0\t0\n1\t1\n2\t1\n3\t2\n4\t3\n5\t5\n6\t8\n7\t13\n8\t21\n9\t34\n10\t55\n11\t89\n12\t144\n13\t233\n14\t377\n15\t610\n",
        &[(b'I', 16), (b'A', 987), (b'B', 1597), (b'C', 1597)],
    );
}

#[test]
fn factorials() {
    check_example(
        include_str!("../examples/factorials.bas"),
        "",
        "N\tFACTORIAL\n0\t1\n1\t1\n2\t2\n3\t6\n4\t24\n5\t120\n6\t720\n7\t5040\n",
        &[(b'N', 8), (b'F', 5040), (b'I', 8)],
    );
}

#[test]
fn multiplication() {
    check_example(
        include_str!("../examples/multiplication.bas"),
        "",
        "MULTIPLICATION TABLE: 1 TO 6\n1\t2\t3\t4\t5\t6\t\n2\t4\t6\t8\t10\t12\t\n3\t6\t9\t12\t15\t18\t\n4\t8\t12\t16\t20\t24\t\n5\t10\t15\t20\t25\t30\t\n6\t12\t18\t24\t30\t36\t\n",
        &[(b'I', 7), (b'J', 7)],
    );
}

#[test]
fn gcd() {
    check_example(
        include_str!("../examples/gcd.bas"),
        "252\n105\n",
        "First positive integer?\n? Second positive integer?\n? GCD = 21\n",
        &[(b'A', 21), (b'B', 0), (b'R', 0)],
    );
}

#[test]
fn gcd_rejects_zero() {
    check_example(
        include_str!("../examples/gcd.bas"),
        "0\n5\n",
        "First positive integer?\n? Second positive integer?\n? Use integers from 1 to 32767.\n",
        &[(b'A', 0), (b'B', 5)],
    );
}

#[test]
fn sort() {
    check_example(
        include_str!("../examples/sort.bas"),
        "",
        "1\n2\n3\n5\n7\n9\n",
        &[(b'I', 6), (b'J', 1), (b'A', 0)],
    );
}

#[test]
fn greeting() {
    check_example(
        include_str!("../examples/greeting.bas"),
        "Ada\n",
        "What is your name?\n? Hello, Ada!\nGreeting length: 11\nArray elements: 4\nLast square: 9\n",
        &[(b'I', 4)],
    );
}

#[test]
fn greeting_with_an_empty_name() {
    check_example(
        include_str!("../examples/greeting.bas"),
        "\n",
        "What is your name?\n? Hello, friend!\nGreeting length: 14\nArray elements: 7\nLast square: 36\n",
        &[(b'I', 7)],
    );
}

#[test]
fn data_table() {
    check_example(
        include_str!("../examples/data_table.bas"),
        "",
        "Monthly units\nCount: 12\nTotal: 243\nTarget: 240\n",
        &[(b'I', 12), (b'S', 243), (b'V', 240)],
    );
}

#[test]
fn memory() {
    check_example(
        include_str!("../examples/memory.bas"),
        "",
        "256\t0\n257\t1\n258\t4\n259\t9\n260\t16\n261\t25\n262\t36\n263\t49\n",
        &[(b'I', 8)],
    );
}
