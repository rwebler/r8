//! Retained 40 by 24 text console used by the reference runner.
#![allow(
    clippy::arithmetic_side_effects,
    reason = "cursor offsets and text-cell dimensions are bounded by the 40 by 24 page"
)]
use crate::video::{TEXT_BYTES, Video};
use std::{
    cell::RefCell,
    io::{self, Write},
    rc::Rc,
};

/// Reference console state shared by output, keyboard editing, and rendering.
pub struct ScreenConsole {
    pub video: Rc<RefCell<Video>>,
    offset: usize,
    /// True while the reference runner waits for a keyboard line.
    pub cursor_visible: bool,
}

impl ScreenConsole {
    #[must_use]
    pub fn new(video: Rc<RefCell<Video>>) -> Self {
        Self {
            video,
            offset: 0,
            cursor_visible: false,
        }
    }
    #[must_use]
    pub fn cursor_cell(&self) -> usize {
        self.offset / 2
    }
    pub fn reset_cursor(&mut self) {
        self.offset = 0;
    }
    pub fn set_cursor(&mut self, row: usize, column: usize) {
        self.offset = (row * 40 + column) * 2;
    }
    pub fn reveal(&mut self) {
        self.video.borrow_mut().write(0xff30, 0);
    }
    fn attribute(&self) -> u8 {
        let video = self.video.borrow();
        (video.paper << 4) | video.ink
    }
    fn cell(&mut self, byte: u8) {
        let attr = self.attribute();
        let mut video = self.video.borrow_mut();
        video.text[self.offset] = byte;
        video.text[self.offset + 1] = attr;
        self.offset += 2;
        if self.offset == TEXT_BYTES {
            video.text.copy_within(80..TEXT_BYTES, 0);
            for cell in video.text[TEXT_BYTES - 80..].chunks_exact_mut(2) {
                cell.copy_from_slice(&[b' ', attr]);
            }
            self.offset = TEXT_BYTES - 80;
        }
    }
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\r' => {}
            b'\n' => {
                let remaining = 40 - self.offset / 2 % 40;
                for _ in 0..remaining {
                    self.cell(b' ');
                }
            }
            b'\t' => self.cell(b' '),
            0x20..=0xff => self.cell(byte),
            _ => {}
        }
    }
    pub fn backspace(&mut self) {
        if self.offset >= 2 {
            self.offset -= 2;
            let attr = self.attribute();
            let mut video = self.video.borrow_mut();
            video.text[self.offset] = b' ';
            video.text[self.offset + 1] = attr;
        }
    }
}

/// Output adapter for the reference interpreter. File streams use their own I/O.
pub struct ScreenWriter(pub Rc<RefCell<ScreenConsole>>);
impl Write for ScreenWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        for &byte in bytes {
            self.0.borrow_mut().write_byte(byte);
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wraps_scrolls_and_preserves_picture_and_attributes() {
        let video = Rc::new(RefCell::new(Video::default()));
        video.borrow_mut().picture[0] = 0x40;
        video.borrow_mut().mode = 1;
        let mut screen = ScreenConsole::new(Rc::clone(&video));
        for _ in 0..24 {
            screen.write_byte(b'X');
            screen.write_byte(b'\n');
        }
        video.borrow_mut().ink = 2;
        video.borrow_mut().paper = 4;
        for _ in 0..40 {
            screen.write_byte(b'Y');
        }
        let video = video.borrow();
        assert_eq!(video.mode, 1);
        assert_eq!(video.picture[0], 0x40);
        assert_eq!(&video.text[..2], &[b'X', 0x67]);
        assert_eq!(
            &video.text[TEXT_BYTES - 160..TEXT_BYTES - 158],
            &[b'Y', 0x42]
        );
    }
}
