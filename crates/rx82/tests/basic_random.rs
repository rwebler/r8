#![cfg(test)]
#![allow(clippy::unwrap_used, reason = "test assertions")]
use rx82::{basic::Basic, native, random::RandomDevice, system::Device};

fn session(source: &str, native_mode: bool, attached: bool) -> String {
    let mut input = source.as_bytes();
    let mut output = Vec::new();
    if native_mode {
        let devices: Vec<Box<dyn Device>> = if attached {
            vec![Box::new(RandomDevice::with_seed(42))]
        } else {
            Vec::new()
        };
        native::interact_with_devices(None, &mut input, &mut output, devices).unwrap();
    } else {
        let mut basic = Basic::default();
        if attached {
            basic.attach_random(RandomDevice::with_seed(42));
        }
        basic.interact(&mut input, &mut output).unwrap();
    }
    String::from_utf8(output).unwrap()
}

#[test]
fn bounded_random_matches_across_interpreters_and_repeats_with_seed() {
    let source = "10 RANDOMIZE -1234\n20 FOR I=1 TO 12\n30 PRINT RND(1),RND(2),RND(6),RND(257),RND(16385),RND(32767)\n40 NEXT I\nRUN\nRUN\nQUIT\n";
    let reference = session(source, false, true);
    let native = session(source, true, true);
    let results = |output: &str| -> Vec<String> {
        output
            .lines()
            .filter(|line| line.contains('\t'))
            .map(|line| line.trim_start_matches("> ").to_owned())
            .collect()
    };
    let reference = results(&reference);
    assert_eq!(reference, results(&native), "{native}");
    assert_eq!(reference.len(), 24);
    for (first, second) in reference.iter().take(12).zip(reference.iter().skip(12)) {
        assert_eq!(first, second);
        for (number, bound) in first.split('\t').zip([1, 2, 6, 257, 0x4001, i16::MAX]) {
            let number = number.parse::<i16>().unwrap();
            assert!(number >= 0 && number < bound);
        }
    }
}

#[test]
fn peek_poke_and_basic_functions_share_one_device() {
    for native_mode in [false, true] {
        let output = session(
            "PRINT PEEK(65313)\nPRINT PEEK(65312),PEEK(65312),PEEK(65312)\nPOKE 65314,42\nPOKE 65315,0\nPRINT RND(6)+1,RND(32767),RND(1)\nRANDOMIZE 42\nPRINT RND(6)+1,RND(32767),RND(1)\nRANDOMIZE 42\nNEW\nPRINT PEEK(65312),PEEK(65312)\nQUIT\n",
            native_mode,
            true,
        );
        assert!(output.contains("> 1\n"), "{output}");
        assert!(output.contains("0\t169\t28\n"), "{output}");
        assert_eq!(output.matches("2\t7385\t0\n").count(), 2, "{output}");
        assert!(output.contains("0\t169\n"), "{output}");
        assert!(!output.contains('?'), "{output}");
    }
}

#[test]
fn invalid_arguments_leave_the_sequence_unchanged() {
    for native_mode in [false, true] {
        let output = session(
            "PRINT RND(0)\nPRINT RND(-1)\nRANDOMIZE 7 8\nPRINT RND(6)+1,RND(32767)\nQUIT\n",
            native_mode,
            true,
        );
        assert_eq!(
            output
                .to_ascii_uppercase()
                .matches("INVALID RANDOM BOUND")
                .count(),
            2,
            "{output}"
        );
        assert!(output.contains("2\t7385\n"), "{output}");
    }
}

#[test]
fn absent_device_reports_a_basic_error() {
    for native_mode in [false, true] {
        let output = session(
            "PRINT RND(6)\nRANDOMIZE 42\nRANDOMIZE\nQUIT\n",
            native_mode,
            false,
        );
        assert_eq!(
            output
                .to_ascii_uppercase()
                .matches("RANDOM DEVICE NOT AVAILABLE")
                .count(),
            3,
            "{output}"
        );
    }
}

#[test]
fn fresh_entropy_and_nested_expressions_work() {
    for native_mode in [false, true] {
        let output = session(
            "RANDOMIZE\nPRINT PEEK(65313)\nRANDOMIZE 0\nPRINT RND(RND(1)+1)\nRANDOMIZE 1\nPRINT RND(RND(1)+1)\nQUIT\n",
            native_mode,
            true,
        );
        assert!(!output.contains('?'), "{output}");
        assert!(output.contains("> 1\n"), "{output}");
        assert_eq!(output.matches("> 0\n").count(), 2, "{output}");
    }
}

#[test]
fn dice_example_produces_repeatable_rolls() {
    let commands = format!("{}\nRUN\nQUIT\n", include_str!("../examples/dice.bas"));
    for native_mode in [false, true] {
        let output = session(&commands, native_mode, true);
        assert!(
            output.contains("Ten rolls of a six-sided die:\n2\n6\n2\n1\n4\n6\n1\n1\n5\n4\n"),
            "{output}"
        );
        assert!(!output.contains('?'), "{output}");
    }
}

#[test]
fn randomize_updates_seed_registers_used_by_poke() {
    for native_mode in [false, true] {
        let output = session(
            "RANDOMIZE 42\nPRINT PEEK(65314)\nPOKE 65315,1\nA=RND(32767)\nRANDOMIZE 298\nPRINT A-RND(32767)\nQUIT\n",
            native_mode,
            true,
        );
        assert!(output.contains("> 42\n"), "{output}");
        assert!(output.contains("> 0\n"), "{output}");
        assert!(!output.contains('?'), "{output}");
    }
}
