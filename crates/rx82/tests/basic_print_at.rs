//! Positioned screen output on both BASIC runners.
#![cfg(test)]
#![allow(clippy::unwrap_used, reason = "test assertions")]
use rx82::{
    basic::Basic,
    native::{self, ConsoleRoute},
    screen_console::{ScreenConsole, ScreenWriter},
};
use std::{cell::RefCell, rc::Rc};

#[test]
fn print_at_positions_text_and_retains_picture_mode() {
    let source = "10 SCREEN 1\n20 PRINT AT 0,0;\"X\";\n30 PRINT AT 23,38;\"Y\";\n40 END\n";
    let mut reference = Basic::default();
    let screen = Rc::new(RefCell::new(ScreenConsole::new(Rc::clone(
        &reference.video,
    ))));
    reference.screen_console = Some(Rc::clone(&screen));
    reference.load(source).unwrap();
    reference
        .run(&mut b"".as_slice(), &mut ScreenWriter(screen))
        .unwrap();
    let (mut sys, console) = native::machine_with_console(ConsoleRoute::Screen);
    console
        .borrow_mut()
        .input
        .extend(format!("{source}RUN\n").bytes());
    console.borrow_mut().eof = true;
    let mut ran = false;
    for _ in 0..50_000_000_u32 {
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
    assert_eq!(reference.video.borrow().mode, 1);
    for video in [&*reference.video.borrow(), &*sys.video.borrow()] {
        assert_eq!(video.mode, 1);
        assert_eq!(video.text[0], b'X');
        assert_eq!(video.text[23 * 80 + 38 * 2], b'Y');
    }
}

#[test]
fn print_at_requires_screen_and_validates_before_cursor_change() {
    let mut reference = Basic::default();
    reference.load("10 PRINT AT 1,1;\"X\"").unwrap();
    assert!(reference.run(&mut b"".as_slice(), &mut Vec::new()).is_err());
    let mut output = Vec::new();
    native::interact(
        None,
        &mut "PRINT AT 1,1;\"X\"\nQUIT\n".as_bytes(),
        &mut output,
    )
    .unwrap();
    assert!(
        String::from_utf8(output)
            .unwrap()
            .contains("SCREEN CONSOLE REQUIRED")
    );
}
