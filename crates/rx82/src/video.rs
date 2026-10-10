//! Video registers, retained pages, and deterministic frame timing.
#![allow(
    clippy::arithmetic_side_effects,
    reason = "fixed 320x192 surface and bounded register coordinates"
)]
use crate::{bus::Bus, system::Device};
use std::{cell::RefCell, rc::Rc};

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 192;
pub const TEXT_BYTES: usize = 1920;
pub const PICTURE_BYTES: usize = 3840;
pub const COLORS: [u32; 16] = [
    0x000000, 0xffffff, 0xff0000, 0x00ffff, 0xff00ff, 0x00ff00, 0x0000ff, 0xffff00, 0x000000,
    0x7f7f7f, 0x7f0000, 0x007f7f, 0x7f007f, 0x007f00, 0x00007f, 0x7f7f00,
];

pub struct Video {
    pub text: [u8; TEXT_BYTES],
    pub picture: [u8; PICTURE_BYTES],
    pub mode: u8,
    pub pointer: u16,
    pub ink: u8,
    pub paper: u8,
    pub x: u8,
    pub y: u8,
    pub slot2: u8,
    pub slot3: u8,
    pub phase: u32,
    serviced: bool,
    value: u8,
}

impl Default for Video {
    fn default() -> Self {
        let mut text = [0; TEXT_BYTES];
        for cell in text.chunks_exact_mut(2) {
            cell.copy_from_slice(&[0x20, 0x67]);
        }
        Self {
            text,
            picture: [0; PICTURE_BYTES],
            mode: 0,
            pointer: 0,
            ink: 7,
            paper: 6,
            x: 0,
            y: 0,
            slot2: 2,
            slot3: 4,
            phase: 0,
            serviced: false,
            value: 0,
        }
    }
}

impl Video {
    pub fn advance(&mut self, ticks: u64) {
        if ticks == 1 {
            self.phase += 60;
            if self.phase >= 4_000_000 {
                self.phase -= 4_000_000;
            }
            return;
        }
        self.phase = ((u128::from(self.phase) + u128::from(ticks) * 60) % 4_000_000) as u32;
    }
    pub fn vblank(&self) -> bool {
        self.phase >= 3_600_000
    }
    pub fn read(&mut self, address: u16) -> Option<u8> {
        Some(match address {
            0xff30 => self.mode | (u8::from(self.vblank()) << 7),
            0xff31 => self.pointer as u8,
            0xff32 => (self.pointer >> 8) as u8,
            0xff33 => {
                let offset = usize::from(self.pointer);
                let value = if self.mode == 0 {
                    self.text.get(offset)
                } else {
                    self.picture.get(offset)
                }
                .copied()
                .unwrap_or(0);
                self.pointer = self.pointer.wrapping_add(1);
                value
            }
            0xff34 => self.ink,
            0xff35 => self.paper,
            0xff36 => self.x,
            0xff37 => self.y,
            0xff38 => self.slot2,
            0xff39 => self.slot3,
            0xff3a..=0xff3f => 0,
            _ => return None,
        })
    }
    pub fn write(&mut self, address: u16, value: u8) -> bool {
        match address {
            0xff30 => {
                if value <= 1 {
                    self.mode = value;
                    self.pointer = 0;
                }
            }
            0xff31 => self.pointer = (self.pointer & 0xff00) | u16::from(value),
            0xff32 => self.pointer = (self.pointer & 0x00ff) | (u16::from(value) << 8),
            0xff33 => {
                let offset = usize::from(self.pointer);
                if self.mode == 0 {
                    if let Some(byte) = self.text.get_mut(offset) {
                        *byte = value;
                    }
                } else if let Some(byte) = self.picture.get_mut(offset) {
                    *byte = value;
                }
                self.pointer = self.pointer.wrapping_add(1);
            }
            0xff34 => self.ink = value & 15,
            0xff35 => self.paper = value & 15,
            0xff36 => self.x = value,
            0xff37 => self.y = value,
            0xff38 => self.slot2 = value & 15,
            0xff39 => self.slot3 = value & 15,
            0xff3a => {
                if self.mode == 1 && self.x < 160 && self.y < 96 {
                    let offset = usize::from(self.y) * 40 + usize::from(self.x) / 4;
                    let shift = 6 - (self.x % 4) * 2;
                    self.picture[offset] =
                        (self.picture[offset] & !(3 << shift)) | ((value & 3) << shift);
                }
            }
            0xff3b..=0xff3f => {}
            _ => return false,
        }
        true
    }
    pub fn pixel_color(&self, x: usize, y: usize) -> u32 {
        if self.mode == 1 {
            let x = x / 2;
            let y = y / 2;
            let byte = self.picture[y * 40 + x / 4];
            let slot = (byte >> (6 - (x % 4) * 2)) & 3;
            COLORS[usize::from([self.paper, self.ink, self.slot2, self.slot3][usize::from(slot)])]
        } else {
            let offset = (y / 8 * 40 + x / 8) * 2;
            let glyph = glyph(self.text[offset]);
            let bit = (glyph[y % 8] >> (7 - x % 8)) & 1;
            let attr = self.text[offset + 1];
            COLORS[usize::from(if bit == 0 { attr >> 4 } else { attr & 15 })]
        }
    }
    pub fn render(&self, frame: &mut [u32]) {
        for (index, pixel) in frame.iter_mut().take(WIDTH * HEIGHT).enumerate() {
            *pixel = self.pixel_color(index % WIDTH, index / WIDTH);
        }
    }
}

fn glyph(code: u8) -> [u8; 8] {
    if (0x80..=0x8f).contains(&code) {
        let bits = code & 15;
        let top = (if bits & 1 != 0 { 0xf0 } else { 0 }) | (if bits & 2 != 0 { 0x0f } else { 0 });
        let bottom =
            (if bits & 4 != 0 { 0xf0 } else { 0 }) | (if bits & 8 != 0 { 0x0f } else { 0 });
        return [top, top, top, top, bottom, bottom, bottom, bottom];
    }
    const BLOCKS: [[u8; 8]; 16] = [
        [0x18; 8],
        [0, 0, 0, 255, 255, 0, 0, 0],
        [0x18, 0x18, 0x18, 255, 255, 0x18, 0x18, 0x18],
        [0, 0, 0, 0x1f, 0x1f, 0x18, 0x18, 0x18],
        [0, 0, 0, 0xf8, 0xf8, 0x18, 0x18, 0x18],
        [0x18, 0x18, 0x18, 0x1f, 0x1f, 0, 0, 0],
        [0x18, 0x18, 0x18, 0xf8, 0xf8, 0, 0, 0],
        [0x18, 0x18, 0x18, 0x1f, 0x1f, 0x18, 0x18, 0x18],
        [0x18, 0x18, 0x18, 0xf8, 0xf8, 0x18, 0x18, 0x18],
        [0, 0, 0, 255, 255, 0x18, 0x18, 0x18],
        [0x18, 0x18, 0x18, 255, 255, 0, 0, 0],
        [0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55],
        [255, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 255],
        [0x7e, 0x42, 0x42, 0x42, 0x4a, 0x42, 0x42, 0x7e],
        [0x7e, 0x46, 0x4a, 0x52, 0x62, 0x42, 0x42, 0x7e],
        [255, 0, 255, 0, 255, 0, 255, 0],
    ];
    if (0x90..=0x9f).contains(&code) {
        BLOCKS[usize::from(code - 0x90)]
    } else {
        crate::video_font::ascii(code)
    }
}

impl Device for Video {
    fn tick(&mut self, bus: &mut Bus) {
        let next = bus.pending_write.is_some();
        if !bus.mem || !(0xff30..=0xff3f).contains(&bus.addr) {
            self.serviced = false;
            return;
        }
        if !self.serviced {
            if bus.write {
                self.write(bus.addr, bus.data);
            } else {
                self.value = self.read(bus.addr).unwrap_or(0);
            }
            self.serviced = true;
        }
        if !bus.write {
            bus.write_data(self.value);
        }
        if next {
            self.serviced = false;
        }
    }
}

impl Device for Rc<RefCell<Video>> {
    fn tick(&mut self, bus: &mut Bus) {
        self.borrow_mut().tick(bus);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_pages_pointer_and_packed_pixels() {
        let mut video = Video::default();
        assert_eq!(&video.text[..4], &[0x20, 0x67, 0x20, 0x67]);
        video.write(0xff33, b'A');
        video.write(0xff33, 0x70);
        assert_eq!(video.text[0], b'A');
        video.write(0xff30, 1);
        for x in 0..4 {
            video.write(0xff36, x);
            video.write(0xff3a, x);
        }
        assert_eq!(video.picture[0], 0x1b);
        assert_eq!(video.pixel_color(0, 0), COLORS[6]);
        assert_eq!(video.pixel_color(2, 0), COLORS[7]);
        assert_eq!(video.pixel_color(4, 0), COLORS[2]);
        video.write(0xff38, 5);
        assert_eq!(video.pixel_color(4, 0), COLORS[5]);
        video.write(0xff30, 0);
        assert_eq!(&video.text[..2], &[b'A', 0x70]);
        video.write(0xff31, 0xff);
        video.write(0xff32, 0xff);
        assert_eq!(video.read(0xff33), Some(0));
        assert_eq!(video.pointer, 0);
        video.write(0xff3b, 0xff);
        assert_eq!(video.read(0xff3b), Some(0));
    }
    #[test]
    fn vblank_phase_and_text_render() {
        let mut video = Video::default();
        video.write(0xff33, b'A');
        video.write(0xff33, 0x70);
        assert!(video.text[0] == b'A');
        assert!(glyph(b'A').iter().any(|&row| row != 0));
        video.advance(59_999);
        assert!(!video.vblank());
        video.advance(1);
        assert!(video.vblank());
        video.advance(6_667);
        assert!(!video.vblank());
    }
    #[test]
    fn held_data_bus_read_increments_once() {
        let mut video = Video::default();
        let mut bus = Bus {
            addr: 0xff33,
            mem: true,
            ..Bus::default()
        };
        for _ in 0..5 {
            video.tick(&mut bus);
            bus.reconcile();
            assert_eq!(bus.data, 0x20);
            assert_eq!(video.pointer, 1);
        }
        bus.mem = false;
        video.tick(&mut bus);
        bus.mem = true;
        video.tick(&mut bus);
        bus.reconcile();
        assert_eq!(bus.data, 0x67);
        assert_eq!(video.pointer, 2);
    }
    #[test]
    fn block_glyph_rows_are_fixed() {
        assert_eq!(glyph(0x80), [0; 8]);
        assert_eq!(glyph(0x81), [0xf0, 0xf0, 0xf0, 0xf0, 0, 0, 0, 0]);
        assert_eq!(glyph(0x8f), [0xff; 8]);
        assert_eq!(
            glyph(0x9c),
            [0xff, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0xff]
        );
        assert_eq!(glyph(0x7f), [0; 8]);
        assert_eq!(glyph(0xa0), [0; 8]);
    }
}
