//! Deterministic four-channel sound chip at FF40..FF47.
#![allow(
    clippy::arithmetic_side_effects,
    reason = "fixed channel and register bounds; wide elapsed-time math"
)]
use crate::{bus::Bus, system::Device};
use std::collections::VecDeque;
use std::{cell::RefCell, rc::Rc};

const SYSTEM_HZ: u64 = 4_000_000;
const CHIP_HZ: u64 = 1_789_773;
const SAMPLE_HZ: u64 = 48_000;
const MAX_SAMPLES: usize = 8192;

pub struct Sound {
    pub registers: [u8; 8],
    pub lfsr: u32,
    pub samples: VecDeque<i16>,
    counter: [u32; 4],
    positive: [bool; 3],
    chip_remainder: u64,
    sample_remainder: u64,
    serviced: bool,
    value: u8,
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            registers: [0; 8],
            lfsr: 0x1ffff,
            samples: VecDeque::with_capacity(MAX_SAMPLES),
            counter: [0; 4],
            positive: [true; 3],
            chip_remainder: 0,
            sample_remainder: 0,
            serviced: false,
            value: 0,
        }
    }
}

impl Sound {
    pub fn read(&self, address: u16) -> Option<u8> {
        self.registers
            .get(usize::from(address.checked_sub(0xff40)?))
            .copied()
    }
    pub fn write(&mut self, address: u16, value: u8) -> bool {
        let Some(index) = address.checked_sub(0xff40).filter(|&index| index < 8) else {
            return false;
        };
        let index = usize::from(index);
        let value = if index == 1 || index == 3 || index == 5 {
            value & 15
        } else {
            value
        };
        if index < 6 {
            self.counter[index / 2] = 0;
            self.positive[index / 2] = true;
        }
        if index == 6 && (self.registers[6] & 31) != (value & 31) {
            self.counter[3] = 0;
            self.lfsr = 0x1ffff;
        }
        self.registers[index] = value;
        true
    }
    fn chip_tick(&mut self) {
        for channel in 0..3 {
            let period = u32::from(self.registers[channel * 2])
                | (u32::from(self.registers[channel * 2 + 1]) << 8);
            if period == 0 {
                self.counter[channel] = 0;
                self.positive[channel] = true;
            } else {
                self.counter[channel] += 1;
                if self.counter[channel] >= 8 * period {
                    self.counter[channel] = 0;
                    self.positive[channel] = !self.positive[channel];
                }
            }
        }
        let period = u32::from(self.registers[6] & 31);
        if period == 0 {
            self.counter[3] = 0;
            self.lfsr = 0x1ffff;
        } else {
            self.counter[3] += 1;
            if self.counter[3] >= 16 * period {
                self.counter[3] = 0;
                let feedback = (self.lfsr ^ (self.lfsr >> 3)) & 1;
                self.lfsr = (self.lfsr >> 1) | (feedback << 16);
            }
        }
    }
    fn sample(&self) -> i16 {
        let mut sum = 0_i32;
        for channel in 0..3 {
            let period = self.registers[channel * 2] != 0 || self.registers[channel * 2 + 1] != 0;
            let volume = i32::from((self.registers[7] >> (channel * 2)) & 3);
            if period && self.registers[6] & (1 << (channel + 5)) == 0 {
                sum += if self.positive[channel] {
                    volume
                } else {
                    -volume
                };
            }
        }
        if self.registers[6] & 31 != 0 {
            let volume = i32::from(self.registers[7] >> 6);
            sum += if self.lfsr & 1 != 0 { volume } else { -volume };
        }
        (sum * 32767 / 12) as i16
    }
    pub fn advance(&mut self, ticks: u64) {
        // Advance at chip edges and sample boundaries. This preserves output
        // regardless of how the caller chunks elapsed system time.
        if self.registers[..6].iter().all(|&byte| byte == 0) && self.registers[6] & 31 == 0 {
            self.chip_remainder = ((u128::from(self.chip_remainder)
                + u128::from(ticks) * u128::from(CHIP_HZ))
                % u128::from(SYSTEM_HZ)) as u64;
            let elapsed =
                u128::from(self.sample_remainder) + u128::from(ticks) * u128::from(SAMPLE_HZ);
            let samples = elapsed / u128::from(SYSTEM_HZ);
            self.sample_remainder = (elapsed % u128::from(SYSTEM_HZ)) as u64;
            for _ in 0..samples.min(MAX_SAMPLES as u128) {
                if self.samples.len() == MAX_SAMPLES {
                    self.samples.pop_front();
                }
                self.samples.push_back(0);
            }
            return;
        }
        for _ in 0..ticks {
            self.chip_remainder += CHIP_HZ;
            if self.chip_remainder >= SYSTEM_HZ {
                self.chip_remainder -= SYSTEM_HZ;
                self.chip_tick();
            }
            self.sample_remainder += SAMPLE_HZ;
            if self.sample_remainder >= SYSTEM_HZ {
                self.sample_remainder -= SYSTEM_HZ;
                if self.samples.len() == MAX_SAMPLES {
                    self.samples.pop_front();
                }
                let sample = self.sample();
                self.samples.push_back(sample);
            }
        }
    }
    pub fn drain_samples(&mut self) -> impl Iterator<Item = i16> + '_ {
        self.samples.drain(..)
    }
}

impl Device for Sound {
    fn tick(&mut self, bus: &mut Bus) {
        let next = bus.pending_write.is_some();
        if !bus.mem || !(0xff40..=0xff47).contains(&bus.addr) {
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

impl Device for Rc<RefCell<Sound>> {
    fn tick(&mut self, bus: &mut Bus) {
        self.borrow_mut().tick(bus);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn masks_periods_noise_and_chunking() {
        let mut a = Sound::default();
        let mut b = Sound::default();
        for sound in [&mut a, &mut b] {
            sound.write(0xff40, 172);
            sound.write(0xff41, 0xf1);
            sound.write(0xff46, 0xe3);
            sound.write(0xff47, 0xc3);
            assert_eq!(sound.read(0xff41), Some(1));
        }
        a.advance(100_000);
        for _ in 0..100 {
            b.advance(1000);
        }
        assert_eq!(a.lfsr, b.lfsr);
        assert_eq!(a.samples, b.samples);
        a.write(0xff46, 0xe0);
        assert_eq!(a.lfsr, 0x1ffff);
        a.write(0xff46, 0);
        a.write(0xff47, 0);
        a.advance(100);
        assert!(a.samples.iter().rev().take(1).all(|&sample| sample == 0));
    }
    #[test]
    fn middle_c_period_has_expected_edge_count_and_mute_keeps_phase() {
        let mut sound = Sound::default();
        sound.write(0xff40, 172);
        sound.write(0xff41, 1);
        sound.write(0xff47, 3);
        sound.advance(400_000); // 0.1 second: about 26 square-wave periods.
        let samples: Vec<_> = sound.drain_samples().collect();
        assert_eq!(samples.len(), 4800);
        let edges = samples
            .windows(2)
            .filter(|pair| pair[0].signum() != pair[1].signum())
            .count();
        assert!((50..=54).contains(&edges));
        let polarity = sound.positive[0];
        sound.write(0xff46, 0x20);
        sound.advance(8_000);
        assert!(sound.samples.iter().all(|&sample| sample == 0));
        assert_ne!(sound.positive[0], polarity);
    }
    #[test]
    fn noise_lfsr_has_fixed_sequence_and_mute_bits_do_not_restart_it() {
        let mut sound = Sound::default();
        sound.write(0xff46, 1);
        for expected in [0xffff, 0x7fff, 0x3fff, 0x1fff] {
            for _ in 0..16 {
                sound.chip_tick();
            }
            assert_eq!(sound.lfsr, expected);
        }
        sound.write(0xff46, 0xe1);
        assert_eq!(sound.lfsr, 0x1fff);
        sound.write(0xff46, 2);
        assert_eq!(sound.lfsr, 0x1ffff);
    }
}
