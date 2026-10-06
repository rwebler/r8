//! Optional random-byte device at FF20..FF23, with explicit seed registers.
#![allow(clippy::module_name_repetitions, reason = "public device API")]
use crate::{bus::Bus, system::Device};
use anyhow::Result;

/// Address of the read-only random-byte register.
pub const RANDOM_ADDRESS: u16 = 0xFF20;

/// A xorshift32 generator exposed as a memory-mapped device.
/// Attach before the system ROM in `System::devices`.
/// Writes to FF20 are ignored; a held read transaction produces only one byte.
pub struct RandomDevice {
    seed_low: u8,
    serviced: bool,
    state: u32,
    status: u8,
    value: u8,
}

impl RandomDevice {
    /// Creates a generator seeded from the host operating system.
    ///
    /// # Errors
    /// Returns an error if the operating system cannot supply a seed.
    pub fn from_entropy() -> Result<Self> {
        let mut device = Self::with_seed(1);
        device.randomize()?;
        Ok(device)
    }

    /// Advances the generator and returns its high byte.
    pub fn next_byte(&mut self) -> u8 {
        self.state ^= self.state << 13_u8;
        self.state ^= self.state >> 17_u8;
        self.state ^= self.state << 5_u8;
        self.state
            .to_be_bytes()
            .first()
            .copied()
            .unwrap_or_default()
    }

    /// Seeds from the operating system, preserving the sequence on failure.
    ///
    /// # Errors
    /// Returns an error if the operating system cannot supply a seed.
    pub fn randomize(&mut self) -> Result<()> {
        let mut bytes = [0; 4];
        getrandom::fill(&mut bytes).map_err(|error| {
            self.status = 0x81;
            anyhow::anyhow!("cannot seed random device: {error}")
        })?;
        self.reseed(u32::from_le_bytes(bytes));
        Ok(())
    }

    /// Reads a device register, returning None for addresses outside FF20..FF23.
    pub fn read(&mut self, address: u16) -> Option<u8> {
        match address {
            RANDOM_ADDRESS => Some(self.next_byte()),
            0xFF21 => Some(self.status),
            0xFF22 => Some(self.seed_low),
            0xFF23 => Some(0),
            _ => None,
        }
    }

    /// Restarts the sequence; zero is normalized to seed 1.
    pub fn reseed(&mut self, seed: u32) {
        self.state = seed.max(1);
        self.status = 1;
    }

    /// Creates a repeatable generator. Zero is normalized to seed 1.
    #[must_use]
    pub const fn with_seed(seed: u32) -> Self {
        Self {
            seed_low: 0,
            serviced: false,
            state: if seed == 0 { 1 } else { seed },
            status: 1,
            value: 0,
        }
    }

    /// Writes a register and returns whether this address belongs to the device.
    /// FF21=1 requests fresh entropy; FF22 stages a seed's low byte; FF23 commits
    /// its high byte. FF20 writes are ignored. Status bit 7 reports seed failure.
    pub fn write(&mut self, address: u16, value: u8) -> bool {
        match address {
            RANDOM_ADDRESS => {}
            0xFF21 => {
                if value == 1 && self.randomize().is_err() {
                    self.status = 0x81;
                }
            }
            0xFF22 => self.seed_low = value,
            0xFF23 => self.reseed(u32::from(u16::from_le_bytes([self.seed_low, value]))),
            _ => return false,
        }
        true
    }
}

impl Device for RandomDevice {
    fn tick(&mut self, bus: &mut Bus) {
        let next_request = bus.pending_write.is_some();
        if !bus.mem || !(RANDOM_ADDRESS..=0xFF23).contains(&bus.addr) {
            self.serviced = false;
            return;
        }
        if !self.serviced {
            if bus.write {
                self.write(bus.addr, bus.data);
            } else {
                self.value = self.read(bus.addr).unwrap_or_default();
            }
            self.serviced = true;
        }
        if !bus.write {
            bus.write_data(self.value);
        }
        if next_request {
            self.serviced = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::System;

    #[test]
    fn held_bus_reads_advance_once_and_writes_do_not_draw() {
        let mut device = RandomDevice::with_seed(42);
        let mut bus = Bus {
            addr: RANDOM_ADDRESS,
            mem: true,
            ..Bus::default()
        };
        for _ in 0_u8..8 {
            device.tick(&mut bus);
            bus.reconcile();
            assert_eq!(bus.data, 0);
        }
        bus.mem = false;
        device.tick(&mut bus);
        bus.mem = true;
        bus.write = true;
        bus.data = 99;
        device.tick(&mut bus);
        bus.reconcile();
        bus.mem = false;
        device.tick(&mut bus);
        bus.mem = true;
        bus.write = false;
        device.tick(&mut bus);
        bus.reconcile();
        assert_eq!(bus.data, 169);
    }

    #[test]
    #[expect(clippy::unwrap_used, reason = "test")]
    fn guest_reads_consecutive_bytes_and_commits_seed_on_high_write() {
        let mut sys = System {
            turbo: true,
            ..System::default()
        };
        sys.devices.insert(0, Box::new(RandomDevice::with_seed(42)));
        let code = r8asm::assemble("ld cd, 0xFF20\nld a, (cd)\nld 0x0200, a\nld a, (cd)\nld 0x0201, a\nld a, 0x2A\nld 0xFF22, a\nld a, (cd)\nld 0x0202, a\nld a, 0x00\nld 0xFF23, a\nld a, (cd)\nld 0x0203, a\nld a, (cd)\nld 0x0204, a\nhalt").unwrap();
        sys.run_program(&code).unwrap();
        assert_eq!(sys.mem.get(0x0200), 0);
        assert_eq!(sys.mem.get(0x0201), 169);
        assert_eq!(sys.mem.get(0x0202), 28);
        assert_eq!(sys.mem.get(0x0203), 0);
        assert_eq!(sys.mem.get(0x0204), 169);
        assert_eq!(sys.peek_mem(0xFF21), 1);
        assert_eq!(sys.peek_mem(0xFFFE), 0);
        assert_eq!(sys.peek_mem(0xFFFF), 0xC0);
    }

    #[test]
    fn status_and_seed_registers_do_not_consume_random_bytes() {
        let mut device = RandomDevice::with_seed(42);
        assert_eq!(device.read(0xFF21), Some(1));
        assert_eq!(device.read(0xFF22), Some(0));
        assert_eq!(device.read(0xFF23), Some(0));
        assert_eq!(device.read(0xFF24), None);
        assert!(!device.write(0xFF24, 1));
        assert_eq!(device.next_byte(), 0);
        assert_eq!(device.next_byte(), 169);
        assert_eq!(device.next_byte(), 28);
    }

    #[test]
    fn monitor_reads_start_fresh_transactions() {
        let mut sys = System {
            turbo: true,
            ..System::default()
        };
        sys.devices.insert(0, Box::new(RandomDevice::with_seed(42)));
        assert_eq!(sys.peek_mem(0xFF20), 0);
        assert_eq!(sys.peek_mem(0xFF20), 169);
        assert_eq!(sys.peek_mem(0xFF21), 1);
        assert_eq!(sys.peek_mem(0xFF20), 28);
    }
}
