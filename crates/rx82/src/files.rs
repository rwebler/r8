//! Guest byte-stream file device. No BASIC parsing or execution occurs here.
use crate::{bus::Bus, system::Device};
use anyhow::{Context as _, Result, bail, ensure};
use std::{fs, path::PathBuf};

/// File port at FF10..FF14. Commands: read=1, write=2, close=3, abort=4, rewind=5.
/// Status: ready=1, EOF=2, error=128. FF12 appends filename bytes; FF13 transfers
/// file bytes; writing FF14 resets the filename. Writes commit only on close.
#[derive(Default)]
pub struct FilePort {
    bytes: Vec<u8>,
    filename: Vec<u8>,
    position: usize,
    serviced: bool,
    status: u8,
    value: u8,
    writing: bool,
}

impl FilePort {
    fn command(&mut self, command: u8) -> Result<()> {
        match command {
            1 | 2 => {
                ensure!(!self.filename.is_empty(), "empty filename");
                let path = PathBuf::from(core::str::from_utf8(&self.filename)?);
                self.writing = command == 2;
                self.bytes = if self.writing {
                    Vec::new()
                } else {
                    ensure!(fs::metadata(&path)?.len() <= 0x0001_0000, "file too large");
                    fs::read(path)?
                };
                self.position = 0;
                self.status = 1;
            }
            3 => {
                if self.writing {
                    let path = PathBuf::from(core::str::from_utf8(&self.filename)?);
                    fs::write(path, &self.bytes)?;
                }
                self.writing = false;
                self.bytes.clear();
                self.position = 0;
                self.status = 1;
            }
            4 => {
                self.writing = false;
                self.bytes.clear();
                self.position = 0;
                self.status = 1;
            }
            5 => {
                ensure!(!self.writing, "cannot rewind output");
                self.position = 0;
                self.status = 1;
            }
            _ => bail!("unknown file command"),
        }
        Ok(())
    }
    fn transfer(&mut self, address: u16, write: bool, byte: u8) -> Result<u8> {
        match (address, write) {
            (0xFF10, true) => self.command(byte)?,
            (0xFF11, false) => {
                return Ok(self.status
                    | if !self.writing && self.position >= self.bytes.len() {
                        2
                    } else {
                        0
                    });
            }
            (0xFF12, true) => {
                ensure!(self.filename.len() < 240, "filename too long");
                self.filename.push(byte);
            }
            (0xFF13, true) => {
                ensure!(
                    self.writing && self.status == 1,
                    "file not open for writing"
                );
                ensure!(self.bytes.len() < 0x0001_0000, "file too large");
                self.bytes.push(byte);
            }
            (0xFF13, false) => {
                let received = self
                    .bytes
                    .get(self.position)
                    .copied()
                    .context("end of file")?;
                self.position = self.position.saturating_add(1);
                return Ok(received);
            }
            (0xFF14, true) => {
                self.filename.clear();
                self.status = 1;
            }
            _ => {}
        }
        Ok(0)
    }
}

impl Device for FilePort {
    fn tick(&mut self, bus: &mut Bus) {
        let next_request = bus.pending_write.is_some();
        if !bus.mem || !(0xFF10..=0xFF14).contains(&bus.addr) {
            self.serviced = false;
            return;
        }
        if !self.serviced {
            if let Ok(value) = self.transfer(bus.addr, bus.write, bus.data) {
                self.value = value;
            } else {
                self.status = 0x80;
                self.value = 0;
                self.writing = false;
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
