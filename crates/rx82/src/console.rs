//! Memory-mapped guest console: FF00 status, FF01 input, FF02 output.
#![allow(
    clippy::arbitrary_source_item_ordering,
    reason = "group device and shared state"
)]
#![allow(clippy::module_name_repetitions, reason = "public console API")]
#![allow(
    clippy::partial_pub_fields,
    reason = "queues are public; bus handshake is private"
)]
extern crate alloc;
use crate::{bus::Bus, system::Device};
use alloc::collections::VecDeque;
use alloc::rc::Rc;
use core::cell::RefCell;

/// Shared console queues used by a terminal frontend or deterministic tests.
#[derive(Default)]
pub struct ConsoleState {
    /// Input bytes waiting for the guest.
    pub input: VecDeque<u8>,
    /// Captured guest output.
    pub output: Vec<u8>,
    /// Host input has reached EOF.
    pub eof: bool,
    /// Guest polled for input while the queue was empty.
    pub waiting: bool,
}

/// A console device. Status bits: 0 = input ready, 1 = EOF.
#[derive(Default)]
pub struct Console {
    /// Frontend access to the console queues.
    pub shared: Rc<RefCell<ConsoleState>>,
    serviced: bool,
    value: u8,
}

impl Device for Console {
    fn tick(&mut self, bus: &mut Bus) {
        // CPU requests become visible after reconcile. A held bus transaction
        // must consume/emit only one character, even over several CPU cycles.
        let next_request = bus.pending_write.is_some();
        if !bus.mem || !(0xFF00..=0xFF02).contains(&bus.addr) {
            self.serviced = false;
            return;
        }
        if !self.serviced {
            let mut state = self.shared.borrow_mut();
            self.value = match (bus.addr, bus.write) {
                (0xFF00, false) => {
                    state.waiting = state.input.is_empty() && !state.eof;
                    u8::from(!state.input.is_empty()) | (u8::from(state.eof) << 1_u8)
                }
                (0xFF01, false) => state.input.pop_front().unwrap_or_default(),
                (0xFF02, true) => {
                    state.output.push(bus.data);
                    0
                }
                _ => 0,
            };
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
    #[expect(clippy::unwrap_used, reason = "test")]
    fn guest_reads_and_writes_each_byte_once() {
        let console = Console::default();
        let shared = Rc::clone(&console.shared);
        shared.borrow_mut().input.extend(b"AB");
        shared.borrow_mut().eof = true;
        let mut sys = System {
            turbo: true,
            ..System::default()
        };
        sys.devices.insert(0, Box::new(console));
        let code = r8asm::assemble_with_debug("ld cd, 0xFF00\nld a, (cd)\nld 0x0100, a\ninc cd\nld a, (cd)\nld 0xFF02, a\nld a, (cd)\nld 0xFF02, a\nhalt").unwrap();
        sys.run_program(&code).unwrap();
        assert_eq!(sys.mem.get(0x0100), 3);
        assert_eq!(shared.borrow().output, b"AB");
        assert!(shared.borrow().input.is_empty());
    }
}
