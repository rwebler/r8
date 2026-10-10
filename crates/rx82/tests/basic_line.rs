//! Bresenham LINE pixels and validation on both BASIC runners.
#![cfg(test)]
#![allow(clippy::unwrap_used, reason = "test assertions")]
use rx82::{
    basic::Basic,
    native::{self, ConsoleRoute},
};

fn native_picture(source: &str) -> [u8; 3840] {
    let (mut sys, console) = native::machine();
    console
        .borrow_mut()
        .input
        .extend(format!("{source}RUN\n").bytes());
    console.borrow_mut().eof = true;
    let mut ran = false;
    for _ in 0..80_000_000_u32 {
        sys.tick();
        ran |= sys.mem.get(0x0082) != 0;
        if ran && sys.mem.get(0x0082) == 0 {
            break;
        }
    }
    assert!(
        ran && sys.mem.get(0x0082) == 0,
        "guest timed out at {:04X}",
        sys.cpu.pc
    );
    sys.video.borrow().picture
}

#[test]
fn line_matches_reference_in_all_directions() {
    let source = "10 SCREEN 1\n20 LINE 0,0,159,95\n30 LINE 159,0,0,95\n40 LINE 80,70,80,2\n50 LINE 145,47,3,47\n60 LINE 4,4,4,4\n70 END\n";
    let mut reference = Basic::default();
    reference.load(source).unwrap();
    reference.run(&mut b"".as_slice(), &mut Vec::new()).unwrap();
    assert_eq!(native_picture(source), reference.video.borrow().picture);
}

#[test]
fn invalid_line_draws_no_pixels() {
    let source = "10 SCREEN 1\n20 LINE 0,0,160,2\n30 END\n";
    let mut reference = Basic::default();
    reference.load(source).unwrap();
    assert!(reference.run(&mut b"".as_slice(), &mut Vec::new()).is_err());
    assert_eq!(native_picture(source), [0; 3840]);
}

#[test]
fn list_keeps_space_after_line_keyword() {
    let commands = format!("{}LIST\nQUIT\n", include_str!("../examples/line_at.bas"));
    let mut reference = Basic::default();
    let mut reference_output = Vec::new();
    reference
        .interact(&mut commands.as_bytes(), &mut reference_output)
        .unwrap();
    assert!(
        String::from_utf8(reference_output)
            .unwrap()
            .contains("50 LINE 8,8,151,8\n")
    );

    let (mut sys, console) = native::machine();
    console.borrow_mut().input.extend(commands.bytes());
    console.borrow_mut().eof = true;
    for _ in 0..20_000_000_u32 {
        if sys.cpu.halt {
            break;
        }
        sys.tick();
    }
    assert!(sys.cpu.halt, "guest timed out at {:04X}", sys.cpu.pc);
    assert!(
        String::from_utf8(console.borrow().output.clone())
            .unwrap()
            .contains("50 LINE 8,8,151,8\n")
    );
}

#[test]
fn line_example_restores_text_colors_after_q() {
    let commands = format!(
        "COLOR 2,4\n{}RUN\nqQUIT\n",
        include_str!("../examples/line_at.bas")
    );
    let (mut sys, console) = native::machine_with_console(ConsoleRoute::Screen);
    console.borrow_mut().input.extend(commands.bytes());
    console.borrow_mut().eof = true;
    for _ in 0..50_000_000_u32 {
        if sys.cpu.halt {
            break;
        }
        sys.tick();
    }
    assert!(sys.cpu.halt, "guest timed out at {:04X}", sys.cpu.pc);
    let video = sys.video.borrow();
    assert_eq!(video.mode, 0);
    assert_eq!((video.ink, video.paper), (2, 4));
    assert_ne!(video.picture, [0; 3840]);
}
