[![Crate](https://img.shields.io/crates/v/rx82.svg)](https://crates.io/crates/rx82)
[![Docs](https://docs.rs/rx82/badge.svg)](https://docs.rs/rx82)
![CI](https://github.com/bitfield/r8/actions/workflows/ci.yml/badge.svg)
![Audit](https://github.com/bitfield/r8/actions/workflows/audit.yml/badge.svg)
![Maintenance](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)

An emulator for the RX82 fantasy retro computer system, including the R8 8-bit CPU.

> ADRIC: _What do these numbers and letters mean?_\
> DOCTOR: _It's an early version. Instructions have to be punched in by machine code._\
> ADRIC: _Oh, how boring._\
> DOCTOR: _**Boring?**_\
—Doctor Who, _Logopolis_

![](img/RX82.jpg)

# Installation

```sh
cargo install --locked rx82
```

# About

This is an emulator for the RX82 architecture, an imagined home computer system similar to those of the early 1980s, such as the Sinclair ZX81 and Spectrum, the BBC Micro, or the Commodore 64.

The RX82's design is intended not only to evoke fond memories in those of a certain age, but also to help teach the fundamentals of computer systems architecture and computer engineering. It's simpler than historic systems such as the ZX81, because no cost or design compromises are required, but also realistic enough to be useful for learning purposes.

Its central processor is the R8, a fan-fiction CPU design comparable to the Zilog Z80 or the MOS 6502, but again, somewhat simplified for educational purposes.

This crate provides a reference implementation of the RX82 and R8 architectures, and an assembler / disassembler for use with R8 assembly language programs. However, it is intended to be modular, so that you can pick and choose components to build your own systems.

For example, you could use the R8 CPU as part of your own emulator that replaces the RX82 system with something else. Equally, you could use the RX82 system components but replace the CPU with a design of your own, or an emulated real machine such as a 6502.

# Usage

## BASIC

The RX-82 includes a host reference interpreter and an optional native BASIC ROM. The complete language and usage reference is in the [BASIC manual](BASIC.md); runnable programs are in the [examples guide](examples/README.md).

## Assembling R8 source files

Prepare your program in a text file (see _R8 Assembly Language_ below), and run:

```sh
rx82 asm my_prog.asm
```

If the program assembles correctly, this will produce a `my_prog.bin` file you can run with the monitor.

You can also use the standalone [`r8asm`](https://crates.io/crates/r8asm) tool.

## Starting the monitor

To start the monitor in debug (single-step) mode:

```sh
rx82 mon
```

```txt
(C) 1982 RX Computers Ltd.

0xBF00 bytes free. Ready.
```

You can also optionally load and run a binary file (such as one produced by the assembler, for example):

```sh
rx82 mon my_prog.bin
```

To run the binary in single-step mode, use the `--step` switch:

```sh
rx82 mon --step my_prog.bin
```

## Using the monitor

The monitor displays CPU registers and the next instruction before each command.
Addresses are hexadecimal (with an optional `0x` prefix).

| Command | Action |
| :--- | :--- |
| `B address` | Add an instruction breakpoint |
| `B` | List breakpoints |
| `BC address` | Remove a breakpoint |
| `BC` | Clear all breakpoints |
| `G [address]` | Run until a breakpoint, HALT, or guest input request |
| `S [address]` | Execute one instruction |
| `M [address]` | Dump 128 bytes as hex and ASCII |
| `I [text]` | Queue a guest input line, preserving case and spaces |
| `H` | Show help |
| `Q` | Quit |
| Enter | Repeat the last `G`, `M`, or `S`, without its address |

Breakpoints stop **before** the instruction executes. After hitting one, `G`
continues past that instruction once and leaves the breakpoint armed for the
next visit. `S` executes the current instruction even if it has a breakpoint.
Breakpoints persist for the monitor session. A pending store is committed before
control returns to the monitor, so memory and console inspection see its result.

For example, start native BASIC with `--step`, then use:

```text
B C100
G
S
BC C100
G
```

`G` first stops at ROM entry. `S` executes the stack initialization instruction.
The final `G` loads and runs the queued program, displaying its output. A guest
input request returns control with an explanation; use `I text` then `G` to
continue. Input/output transport is available when a guest console is attached,
as it is for native BASIC.

Memory dumps include an ASCII column; nonprintable bytes appear as dots:

```text
> M 1080
1080: 14 00 46 4F 52 20 49 20 3D 20 31 30 20 54 4F 20  |..FOR I = 10 TO |
1090: 30 20 53 54 45 50 20 2D 32 00 00 00 00 00 00 00  |0 STEP -2.......|
...
```

Enter (or `M` without an address) continues at the next 128-byte block. Dumps
wrap from `FFFF` to `0000`. Memory-mapped I/O reads can have device side effects;
use the RAM and ROM ranges when inspecting code, source, and variables.

## Disassembling R8 binary files

Run:

```sh
rx82 dis my_prog.bin
```

This will print the disassembled listing.

# RX82 user's manual

## The RX82 architecture

The RX82 is a single-board computer with one R8 CPU clocked at 4Mhz, 64KiB of static RAM, an 8-bit data bus, and a 16-bit address bus.

## Memory map

| Address | Contents |
| :--- | :---    |
| 0x0000 | Trap/interrupt table |
| 0x0080 | System data area |
| 0x0100 | User RAM |
| 0xC000 | ROM |
| 0xFF00 | System I/O area |
| 0xFFFE | Reset vector |

## Guest console device

The optional `console::Console` device provides byte I/O for native guest
programs. Attach it before the ROM in `System::devices` so these registers take
priority over the ROM mapping. Frontends feed its shared input queue and drain
its output queue; all parsing and computation remain guest instructions.

| Address | Operation |
| :--- | :--- |
| `0xFF00` read | Status: bit 0 = input available, bit 1 = host EOF |
| `0xFF01` read | Consume one input byte (zero if empty) |
| `0xFF02` write | Emit one output byte |

Guests should check input availability before reading data. EOF may be set
while queued bytes remain. The existing `PUTCHAR` trap remains available to
legacy programs. Native console I/O can be tested without a host terminal.

## Random device

The optional `random::RandomDevice` is a separate module implementing `Device`.
The default machine does not attach it. Enable it with `--random-device` on
`basic`, `mon`, or `run`; `--random-seed 42` makes its initial sequence repeatable.
For `mon` and `run`, the frontend attaches it after the stock boot RAM test.
It also works with native BASIC's `--step` and `--break-before-run` options.

```rust
use rx82::{random::RandomDevice, system::System};

let mut sys = System::default();
sys.devices.insert(0, Box::new(RandomDevice::with_seed(42)));
```

Use `RandomDevice::from_entropy()?` for an operating-system seed. The device
uses xorshift32 (shifts 13, 17, 5), returning the high byte after each step.
It is intended for games and simulations. A bus read held across multiple CPU
cycles advances the generator only once; successive load instructions each
advance it. The firmware and CPU instruction set need no changes.

| Address | Read | Write |
| :--- | :--- | :--- |
| `FF20` / 65312 | Next random byte | Ignored |
| `FF21` / 65313 | Status: bit 0 present, bit 7 seed request failed | 1 requests a fresh operating-system seed; other values ignored |
| `FF22` / 65314 | Staged low seed byte | Stage low seed byte |
| `FF23` / 65315 | Zero | Commit high byte and restart with the assembled 16-bit seed |

A successful seed request clears the error flag. A failed entropy request
preserves the old sequence; BASIC reports `RANDOM SEED FAILED` in native mode.
Write both seed bytes, low first, when reseeding through `POKE`. Reading status
or seed registers does not advance the generator. Monitor dumps that include
`FF20` **do** consume a byte, as do direct `PEEK` calls, affecting later results.
With no device installed these addresses read the stock ROM's zero padding.

The default `System` installs video (FF30–FF3F) and sound (FF40–FF47) devices
ahead of ROM. Their state is available through `System::video` and
`System::sound`. The host BASIC interpreter uses the same register models.

## Boot process

At power on, the CPU loads the reset vector at 0xFFFE, which in the RX82 system holds the ROM entry point, 0xC000. Execution begins here and a simple RAM test is performed to find the highest writable address in memory. The stack pointer is initialised to this address.

The trap table is initialised, and all undefined traps are vectored to a single 'undefined trap' handler.

Finally, the interactive monitor is invoked.

## OS traps

The following general-purpose traps are defined:

| Code | Name | Purpose | Inputs |
| :--- | :--- | :--- | :--- |
| 0x20 | PUTCHAR | Print character to terminal | A = ASCII code of character |

## Opcodes

| | -0 | -1 | -2 | -3 | -4 | -5 | -6 | -7 | -8 | -9 | -A | -B | -C | -D | -E | -F |
| :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: |
| 0- | halt | nop | | sec | clc | sei | cli | ret | rti | | | | | | | |
| 1- | ld a, N | ld b, N | ld c, N | ld d, N | ld e, N | ld f, N | ld g, N | ld h, N | ld ab, NN | ld cd, NN | ld ef, NN | ld gh, NN | ld sp, NN | ld R, (RR) | ld R1, R2 | ld R, (RR+N) |
| 2- | ld NN, a | ld NN, b | ld NN, c | ld NN, d | ld NN, e | ld NN, f | ld NN, g | ld NN, h | ld (RR), R | ld (RR), N | | | | | | ld (RR+N), R |
| 3- | inc a | inc b | inc c | inc d | inc e | inc f | inc g | inc h | inc ab | inc cd | inc ef | inc gh | inc sp | inc (RR) | inc (NN) | |
| 4- | dec a | dec b | dec c | dec d | dec e | dec f | dec g | dec h | dec ab | dec cd | dec ef | dec gh | dec sp | dec (RR) | dec (NN) | |
| 5- | add a, N | add b, N | add c, N | add d, N | add e, N | add f, N | add g, N | add h, N | add ab, N | add cd, N | add ef, N | add gh, N | add sp, N | | | add R1, R2 |
| 6- | sub a, N | sub b, N | sub c, N | sub d, N | sub e, N | sub f, N | sub g, N | sub h, N | sub ab, N | sub cd, N | sub ef, N | sub gh, N | sub sp, N | | | sub R1, R2 |
| 7- | cmp a, N | cmp b, N | cmp c, N | cmp d, N | cmp e, N | cmp f, N | cmp g, N | cmp h, N | cmp ab, N | cmp cd, N | cmp ef, N | cmp gh, N | cmp sp, N | | | cmp R1, R2 |
| 8- | and a, N | and b, N | and c, N | and d, N | and e, N | and f, N | and g, N | and h, N | and ab, N | and cd, N | and ef, N | and gh, N | and sp, N | | | and R1, R2 |
| 9- | test a, N | test b, N | test c, N | test d, N | test e, N | test f, N | test g, N | test h, N | test ab, N | test cd, N | test ef, N | test gh, N | test sp, N | | | test R1, R2 |
| A- | or a, N | or b, N | or c, N | or d, N | or e, N | or f, N | or g, N | or h, N | or ab, N | or cd, N | or ef, N | or gh, N | or sp, N | | | or R1, R2 |
| B- | | | | | | | | | | | | | | | | |
| C- | shl R, S | shl R1, R2 | lsr R, S | lsr R1, R2 | | | | | | | | | | | | |
| D- | push a | push b | push c | push d | push e | push f | push g | push h | push ab | push cd | push ef | push gh | push ps | | | |
| E- | pop a | pop b | pop c | pop d | pop e | pop f | pop g | pop h | pop ab | pop cd | pop ef | pop gh | pop ps | | | |
| F- | bra D | beq D | bne D | bcs D | bcc D | bmi D | bpl D | jmp NN | call NN | trap T | | | | | | |

# About the emulator

This is a **cycle-stepped** emulator (sometimes called a “low-level” emulator) that models the whole computer system, including the CPU, devices, bus, and so forth. Unlike a “high-level”, or **instruction-stepped** emulator, where the CPU “owns” all the resources, such as memory, and can manipulate them directly, in a low-level emulator the CPU must read and write signals to the bus like any other device.

This makes it more complicated, since the emulator must model the CPU's internal state (fetch, decode, execute, and so on), the bus signalling, and all the devices, but it's also more realistic and interesting.

If you're interested in writing an emulator, though, it's much easier to get started with a high-level one. You can read a tutorial series on writing a high-level R8 emulator here:

* [Welcome to the machine: emulating a CPU](https://bitfieldconsulting.com/posts/welcome-to-machine)

# See also

* [`r8asm`](https://crates.io/crates/r8asm): An R8 assembler / disassembler.
* [`r8cpu`](https://crates.io/crates/r8cpu): Core types and logic for the R8 architecture.

# Changelog

* **0.6.0** — split into assembler / core / emulator crates, `lsr`, `and R, N`, `add R, N`, `sub R, N`, `jmp NN`, `bcc` / `bcs`, `sec / clc`, `include`, turbo mode, ROM improvements
* **0.5.0** — `org` and `data` directives, traps implemented, `trap`, `rti`, `call`, `ret`, `ld (RR), R`, `ld R1, R2`, `push`, `pop`, `inc/dec (RR)`, `inc/dec (NN)`, `bra` instructions, reset vector, stack pointer, ROM binary, forward labels
* **0.4.0** — `beq`, `bne`, `inc`, `dec`, and `cmp` instructions; zero and carry flags; backward labels, comments
* **0.3.0** — all registers, load immediate and store direct instructions
* **0.2.0** — monitor improvements, add `halt` instruction, add assembler
* **0.1.0** — first release
