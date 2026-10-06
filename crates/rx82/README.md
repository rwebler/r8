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

Start the Rust-hosted BASIC interpreter:

```sh
cargo run -p rx82 -- basic
```

Or run a numbered source file (then exit):

```sh
cargo run -p rx82 -- basic crates/rx82/examples/squares.bas
```

With an installed binary, use `rx82 basic [file.bas]`. This interpreter runs on
the host and does not execute R8 instructions or access emulated memory.

```basic
10 INPUT N
20 LET I = 1
30 PRINT I, I * I
40 I = I + 1
50 IF I < N THEN 30
60 END
RUN
```

Enter numbered lines (1–65535) to insert or replace them; enter a bare line
number to delete it. `LIST` displays the program, `RUN` executes it from the
lowest line number with cleared variables, and `NEW` clears it. `HELP` shows
commands; `QUIT` or end of input exits. Unnumbered statements execute immediately.
Source files contain numbered lines and optional blank lines.

Save and reload programs at the prompt:

```basic
SAVE "squares.bas"
NEW
LOAD "squares.bas"
LIST
RUN
```

`SAVE` writes the current program in line-number order as plain text, replacing
an existing file. `LOAD` replaces the program and clears variables after the file
has been read successfully and its line numbers validated. An unreadable file or
invalid line numbering leaves the current program and variables intact. Statement
syntax is checked when the program runs, as with manually entered lines.
Both commands require a quoted, nonempty filename; spaces and letter case in
filenames are preserved. Relative paths use the process's current directory.
These commands are available at the prompt only. An empty file loads an empty
program; saving an empty program writes an empty file.

The initial dialect supports:

- `DIM A(100)` allocates 101 signed 16-bit elements, indexed 0 through 100.
  Elements start at zero. Use `A(I)` in expressions, assignments, and `INPUT A(I)`.
  Bounds and indices can be expressions; negative or excessive indices are errors.
  Declare each array before use; a second `DIM` for the same array is an error.
  Scalar `A` and array `A(...)` are separate. Arrays are one-dimensional, with
  one declaration per statement and a shared limit of 2,048 elements.
  `RUN`, `NEW`, and successful `LOAD` clear arrays; `SAVE` stores source only.
- `LET name = expression` (the `LET` keyword is optional), and `INPUT name`.
- String variables such as `A$`, assignment (`A$="hello"`), `INPUT A$`,
  concatenation (`A$+"!"`), and parenthesized string expressions.
- `LEN(string)` counts characters; `LEN(array)` counts allocated elements.
  For `DIM A(100)`, `LEN(A)` is **101**, including element zero. Pass the bare
  array name, not an indexed element: `LEN(A(0))` is a type error.
- `PRINT` or `?` with string and integer expressions. Semicolons join
  items; commas insert tabs. A trailing separator suppresses the newline.
- `IF expression comparison expression THEN line` or `THEN statement`, with
  `=`, `<>`, `<`, `<=`, `>`, and `>=` comparisons. Both operands must be the
  same type. Strings compare in case-sensitive ASCII order.
- `FOR name = start TO limit [STEP step]` and `NEXT [name]` in stored programs.
- `DATA` constants, `READ` into variables or array elements, and `RESTORE [line]`.
- `PEEK(address)` reads a byte; `POKE address,value` writes a byte from 0 to 255.
- `GOTO line`, `GOSUB line`, `RETURN`, `END`, `STOP`, and `REM` comments.

Keywords and variable names are case-insensitive; names start with a letter and
contain letters or digits, with a final `$` for strings. Unset numeric variables
read as zero. Numeric values are signed
16-bit integers (-32768 through 32767), with checked arithmetic, `+`, `-`, `*`, `/`, unary signs, and
parentheses. Division truncates toward zero. Errors in a running program report
the line number; interactive errors return to the prompt. Ctrl-C terminates the
process, including an infinite BASIC loop. Subroutine and active loop nesting
are each limited to 256. Line numbers retain their unsigned range of 1–65535;
literal jump targets may use that full range even though numeric expressions
use signed 16-bit values.

### Strings and LEN

```basic
10 INPUT N$
20 G$="Hello, "+N$+"!"
30 PRINT G$;" (";LEN(G$);" characters)"
40 DIM A(10)
50 FOR I=0 TO LEN(A)-1
60 A(I)=I*I
70 NEXT I
```

String variables end in `$` and default to the empty string. `A`, `A$`, and
array `A(...)` are independent. Each string value, including literals and
concatenation results, may contain up to 63 characters: printable ASCII and
tabs. Case and spaces are preserved. `INPUT A$` reads an unquoted line, preserving
leading and trailing spaces; an empty line assigns `""`. Quotes in input are
ordinary characters. Quoted source literals have no escape syntax.

`LEN("")` is zero; `LEN("ab"+"cd")` is four. Its integer result works in
arithmetic, array indices, `DIM`, and loop bounds. `LEN(A)` requires a declared
array even when scalar `A` exists. Integers are not implicitly converted to
strings: use `PRINT "Score: ";N` to print a number alongside text.

`RUN`, `NEW`, and successful `LOAD` clear strings as well as numeric variables
and arrays. Failed string assignments and failed `LOAD` operations preserve
existing values. `SAVE` stores source, including string literals, rather than
runtime values. String arrays are not supported. The native ROM has 26 string
variables (`A$`–`Z$`); the reference interpreter also permits longer names.

### DATA / READ / RESTORE

Use `DATA` to keep a table together, and `READ` to fill its array:

```basic
10 DIM A(5)
20 FOR I=0 TO LEN(A)-1
30 READ A(I)
40 NEXT I
50 PRINT A(0),A(5)
60 END
100 DATA 9,2,7
110 DATA 1,5,3
```

As in the [MSX BASIC reference](https://www.msxarchive.nl/pub/msx/docs/manuals/msxtech.pdf),
`DATA` statements are skipped during execution. `READ` visits their constants
in ascending line-number order, including data after `END`, and continues from
the first unread item on each call. The cursor is shared by the program,
subroutines, and direct-mode commands. Only standalone `DATA` lines contribute
items; each source line still contains one statement.

`READ A,B,A(I),N$` assigns items from left to right. Numeric targets require
unquoted signed decimal integer constants in the range -32768 to 32767;
expressions, floating point, and hexadecimal constants are not supported.
String targets accept quoted or unquoted text, preserving case. Surround text
containing commas, colons, or significant edge spaces with quotes:

```basic
100 DATA MiXeD words,"Hello, world","",-12
```

Unquoted text is trimmed at both ends. Quoted `""` is an empty string; omitted
fields and trailing commas are invalid. Each item is limited to 63 ASCII
characters. DATA syntax is checked when an item is read, so unused items do
not cause runtime errors. Numeric-looking items can be read as text with a
string target; quoted text is not converted to an integer.

`RESTORE` rewinds to the first item. `RESTORE 110` requires an existing line
and starts searching for data at that line, even if it is not itself a `DATA`
statement. The optional line number must be a literal, not an expression.
`RUN`, `NEW`, successful `LOAD`, and numbered-line edits reset the cursor.
`SAVE` preserves the DATA source; failed `LOAD` leaves the cursor intact.

Reading past the available items reports `OUT OF DATA`. A failed read preserves
its destination and leaves that item unread; assignments earlier in the same
`READ` remain in effect. Errors during a program identify the executing `READ`
line. The [table example](examples/data_table.bas) demonstrates mixed data and
`RESTORE`; [sorting](examples/sort.bas) now fills its array from `DATA`.

### PEEK / POKE

```basic
10 POKE 256,42
20 PRINT PEEK(256)
30 POKE 257,PEEK(256)+1
40 PRINT PEEK(257)
```

`PEEK` returns an integer from 0 to 255. `POKE` accepts integer expressions for
the address and value, and rejects values outside 0–255 without writing.
Addresses can be bare unsigned decimal literals from 0 to 65535. Computed
addresses use normal checked signed 16-bit arithmetic, with negative results
interpreted as address bit patterns: `-1` addresses 65535, and `-28672`
addresses 36864 (`9000` hex). For example, use `P=-28672` then `PEEK(P+I)`
to inspect successive bytes in the native array pool. A high unsigned literal
is accepted directly (`PEEK(36864)`), but cannot be part of an arithmetic
expression (`PEEK(36864+I)` overflows). Addresses in BASIC are decimal; monitor
addresses are hexadecimal.

With `--native`, both operations run as R8 loads/stores through the RX-82 bus.
They access actual RAM, ROM, and memory-mapped devices. For example:

```basic
A=4660
PRINT PEEK(768),PEEK(769)
POKE 768,120
PRINT A
```

This reads A's little-endian bytes (52 and 18), then changes A to 4728.
`POKE 65282,65` writes `A` to the console output register. Reading the console
input register with `PEEK(65281)` consumes one input byte. ROM writes have no
effect. Writes to interpreter storage, source records, or the stack change the
running machine and can disrupt it. The unused RAM at `0100`–`01FF` (decimal
256–511) is suitable for small memory experiments; see [memory.bas](examples/memory.bas).

The Rust reference interpreter provides its own zero-filled 64 KiB byte
address space. It has no emulated CPU, ROM, devices, or BASIC variable mapping:
`POKE 768,...` there does not change A. Reference bytes persist across `RUN`,
`NEW`, and `LOAD`, until the interpreter session ends. In native mode these
commands retain unused RAM but reset or replace their usual BASIC storage.

### FOR / NEXT loops

```basic
10 FOR I = 10 TO 0 STEP -2
20 PRINT I
30 NEXT I
```

This prints 10, 8, 6, 4, 2, and 0 on separate lines. `STEP` defaults to 1 and
must be nonzero. Start, limit, and step expressions are evaluated once on entry,
before assigning the start value to the loop variable. Bounds are inclusive.
If the initial value is already past the limit in the step direction, execution
skips to the line after the matching `NEXT`; the variable keeps its start value.

Loops may nest with distinct variables. `NEXT` without a name closes the
innermost loop; a named `NEXT` must match that loop. `FOR` and `NEXT` must be
standalone numbered statements, not immediate commands or `IF ... THEN`
statements. `RUN` tokenizes the program and checks loop pairing before executing
it, including loops in skipped branches. Other syntax is checked as executed.

`NEXT` adds the captured step to the current loop variable, so assignments to
that variable in the body affect iteration. The variable keeps the first value
past the limit after normal completion. Every increment uses checked 16-bit
arithmetic: for example, `FOR I=32767 TO 32767` reports overflow at `NEXT` rather
than wrapping. The same applies to stepping downward past -32768.

`GOTO` out of a loop discards its active loop state; jumping into a loop does
not initialize it. Re-entering its `FOR` line starts it again. `GOSUB` preserves
caller loops, and `RETURN` discards loops started by the subroutine. An already
active loop variable cannot be reused by another loop. `END`, `STOP`, errors,
and a fresh `RUN` discard execution's loop state.

This version has one statement per line, integer variables and arrays, and
string variables. Floating point and string arrays are not implemented.

More runnable programs and a monitor inspection walkthrough are in the
[BASIC examples guide](examples/README.md), including Fibonacci numbers,
factorials, a multiplication table, and Euclid's GCD algorithm.

## Native BASIC ROM

`rx82 basic --native` enters an optional ROM module written in R8 assembly. The Rust
interpreter remains the default/reference. Native parsing, line editing,
variables, and statement execution happen on the emulated CPU; Rust only
transports terminal bytes. `rx82 basic --native file.bas` types the file into
the guest console followed by `RUN` and `QUIT`.

```sh
cargo run -p rx82 -- basic --native
cargo run -p rx82 -- basic --native --step program.bas
cargo run -p rx82 -- basic --native --break-before-run program.bas
```

`--step` opens the monitor at BASIC ROM entry (`D000`), before the source is loaded.
`--break-before-run` first lets the ROM consume the numbered source file, then
opens the monitor with records loaded and `RUN` queued but not executed. Use
`M 1000` to inspect source, `M 0300` to inspect variables, and `G` to continue.
The preload mode rejects unnumbered commands and reports ROM loading errors;
it has a 20-million-cycle loading limit.

Guest console output is displayed during stepping and continuous execution.
When the guest needs keyboard input, execution returns to the monitor at an
instruction boundary. Use `I 5` followed by `G` to answer a numeric `INPUT`, or
`I LIST` / `I RUN` followed by `G` to issue a BASIC command at its prompt. An
empty `I` sends a blank line. `Q` (or host EOF) exits the monitor; `I QUIT` then
`G` asks the guest to halt. Breakpoints are R8 instruction addresses, not BASIC
line numbers; see the monitor commands below.

The native dialect supports `PRINT`, assignment (`LET` optional), integer and
string `INPUT`, string concatenation, `LEN`, `DATA`/`READ`/`RESTORE`, `PEEK`/`POKE`, one-dimensional integer arrays
declared with `DIM`, signed 16-bit expressions (`+ - * /`, parentheses, unary signs), all
six comparisons with `IF ... THEN`, `GOTO`, `GOSUB`/`RETURN`, `FOR`/`NEXT` with
`STEP`, `REM`, `END`/`STOP`, `LIST`, `RUN`, `NEW`, and `QUIT`. Bounds, step
capture, and checked arithmetic follow the reference dialect. Invalid input
reports a specific cause and returns to the prompt. Errors raised during `RUN`
include the current BASIC line number; direct-mode errors omit it.

Native resource limits: variables are single letters A–Z, reset to zero on
`RUN`; line numbers are 1–65535; source lines are limited to 122 bytes; storage
has 256 fixed-size records; 64 subroutine frames and 48 loop frames fit in RAM.
`FOR`/`NEXT` must be standalone statements. Loop matching occurs when the `FOR`
is executed, rather than the reference interpreter's whole-program precheck.
`NEXT` detects missing or mismatched active loops. A `GOTO` outside a loop
discards its frame; subroutines preserve caller loops and discard local loops
on return. `SAVE`/`LOAD` operate at the prompt using quoted host filenames, preserving
spaces and case. `SAVE` writes sorted numbered text and replaces an existing
file on close. `LOAD` validates numbering, control bytes, line lengths, and the
256-nonblank-line limit before replacing program records and clearing variables.
A failed validation or open leaves the old program and variables intact.
Statement syntax is checked during execution. Blank lines, CRLF, and a missing
final newline are accepted. Empty files clear the program. Files are limited
to 64 KiB by the byte-stream device.

Native and reference BASIC use the same text file format; programs must respect
the native dialect's resource limits to run on both. For example:

```basic
LOAD "crates/rx82/examples/countdown.bas"
RUN
SAVE "My Countdown.bas"
```

The native file port does no BASIC parsing: `FF10` accepts commands (1 open for
read, 2 open for write, 3 close/commit, 4 abort, 5 rewind); `FF11` returns status
(bit 0 ready, bit 1 EOF, bit 7 error); writes to `FF12` append filename bytes;
`FF13` transfers data bytes; a write to `FF14` clears the filename. Reads use an
immutable snapshot so the ROM's validation/install passes see identical bytes.
Writes accumulate until close. Relative paths use the host working directory.
The device is attached only to the native BASIC machine, before its ROM.

The original `sys/rx82_rom.asm` and its binary remain unchanged. BASIC is a
separate image mapped at `D000` in the system firmware's unused padding.
`System::install_rom(start, data)` installs modules within `D000`–`FEFF`, rejecting
overlapping modules, nonzero firmware bytes, and images outside that window.
Only the image's actual bytes are mapped. `System::enter_rom(start)` initializes
the CPU at an installed module's entry point, preserving RAM. The native BASIC
frontend uses this interface directly; it does not run the stock boot sequence
first. Ordinary `run` and `mon` machines have no BASIC module installed.
The stock reset vector still points to `C000`; reset enters the system firmware,
not BASIC. This is a fixed address expansion window, not bank switching or
automatic firmware discovery. Modules must be assembled for their load address.

Memory layout: console at `FF00`–`FF02`, BASIC code starting at `D000` (within
`D000`–`FEFF`), original system ROM at `C000` and reset vector at `FFFE`,
line buffer at `0200`, little-endian variable words at `0300`–`0333`,
subroutine frames at `0400`–`04FF`, loop frames at `0500`–`07FF`,
array descriptors at `0800`–`0867`, string slots at `0900`–`0F7F`,
string expression buffers at `0F80`–`0FFF`, program records at `1000`–`8FFF`,
array elements at `9000`–`9FFF`, stack at `A000`–`BFFF`.
Each four-byte array descriptor (A through Z) contains a little-endian base
address and inclusive upper bound. A zero base means undeclared; elements
are little-endian signed words. `M 0800` inspects declarations and `M 9000`
inspects the first allocated array. Strings occupy 64-byte slots with a NUL
terminator: A$ at `0900`, B$ at `0940`, through Z$ at `0F40`. `M 0900`
shows the first two strings in the dump's ASCII column. Each 128-byte record has
a little-endian line number followed by NUL-terminated source; zero marks a free
record. ROM source is `sys/basic_rom.asm`; rebuild its checked-in image with
`cargo run -p rx82 -- asm crates/rx82/sys/basic_rom.asm`. Tests verify image/source
agreement, ROM size, output, and guest RAM contents.

Native multiplication uses `LSR` and conditional addition to process at most
16 multiplier bits. Division aligns the divisor with at most 15 doublings,
then uses `LSR` and conditional subtraction to process at most 16 quotient
bits. Signed results, truncation toward zero, overflow checks, and division
by zero errors are preserved. Tests cover signed boundary cases and compare
R8 cycle counts with the former repeated-addition and repeated-subtraction
routines. Run the isolated comparisons with:

```sh
cargo test -p rx82 native_multiplication_cycle_regression -- --nocapture
cargo test -p rx82 native_division_cycle_regression -- --nocapture
```

These counts include the test harness and arithmetic helpers, but exclude
BASIC parsing and printing.

The native DATA cursor uses little-endian words at `00B4` (last scanned line)
and `00B6` (next item's source address; zero means search the next line).
The ROM parses DATA directly from program records without a separate data copy.

### Native diagnostics

For example, running `10 NEXT I` reports `? NEXT WITHOUT FOR IN LINE 10`.
Running `10 GOTO 999` without line 999 reports `? UNDEFINED LINE IN LINE 10`.
The line suffix identifies the executing statement, not the missing target.

| Message | Cause / remedy |
| :--- | :--- |
| `ARRAY NOT DIMENSIONED` | Declare the array with `DIM` before use. |
| `ARRAY ALREADY DIMENSIONED` | An array may be declared only once per run. |
| `SUBSCRIPT OUT OF RANGE` | Use an index from zero through the declared bound. |
| `ARRAY MEMORY FULL` | All arrays together must fit in 2,048 elements. |
| `TYPE MISMATCH` | Use matching types; `LEN` accepts strings or bare array names. |
| `STRING TOO LONG` | A string value exceeds 63 characters. Shorten it before concatenating. |
| `INVALID STRING CHARACTER` | Use printable ASCII or tabs. |
| `STRING ARRAYS NOT SUPPORTED` | `DIM` currently declares integer arrays only. |
| `OUT OF DATA` | No unread DATA items remain; add data or use `RESTORE`. |
| `INVALID DATA` | A read encountered an empty field or malformed constant delimiter. |
| `BYTE OUT OF RANGE` | The value given to `POKE` must be between 0 and 255. |
| `EXPECTED COMMA` | Separate the `POKE` address and byte value with a comma. |
| `NEXT WITHOUT FOR` | No active loop in this subroutine; enter through its `FOR`. |
| `NEXT MISMATCH` | The variable or closing statement does not match the active loop. |
| `FOR WITHOUT NEXT` | The ROM could not find a closing `NEXT`. |
| `ZERO STEP` | Use a nonzero `STEP`. |
| `FOR VARIABLE ALREADY ACTIVE` | Use distinct variables for nested loops. |
| `RETURN WITHOUT GOSUB` | There is no subroutine return address. |
| `UNDEFINED LINE` | A `GOTO`, `GOSUB`, or conditional jump target does not exist. |
| `PROGRAM FULL` | All 256 program slots are occupied, or a loaded file exceeds the 256-nonblank-line limit. Delete lines before adding more. |
| `GOSUB STACK FULL`, `FOR STACK FULL`, `EXPRESSION TOO DEEP` | Execution exhausted the corresponding stack limit. |
| `INVALID LINE NUMBER` | A source line or target has an invalid number, including zero. |
| `LINE TOO LONG`, `INVALID CHARACTER` | Input exceeds 122 bytes or contains an unsupported control byte. |
| `INTEGER OVERFLOW`, `DIVISION BY ZERO` | Arithmetic exceeded its range or divided by zero. |
| `REQUIRES RUN`, `DIRECT MODE ONLY` | Use the statement in a stored program or at the prompt, respectively. |
| `FOR/NEXT MUST STAND ALONE` | Put the loop statement on its own numbered line, outside `IF ... THEN`. |
| `FILE ERROR` | The host file operation failed. Check its path and permissions. |

Syntax diagnostics identify the missing component: `EXPECTED =`, `EXPECTED )`,
`EXPECTED TO`, `EXPECTED THEN`, `EXPECTED COMPARISON`, `EXPECTED VARIABLE A-Z`,
or `EXPECTED QUOTED FILENAME`. An open quote reports `UNTERMINATED STRING`;
extra text after a complete statement reports `UNEXPECTED INPUT`.

Errors reset the execution stack and return to the prompt without deleting the
program. Failed `LOAD` validation preserves the existing program and variables.
Rejected input lines are discarded through their newline (or EOF), so their
remaining text cannot accidentally become another command. A mismatched `NEXT`
found while scanning a `FOR` reports the opening `FOR` line, where the scan runs.

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
B D000
G
S
BC D000
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
| 0- | halt | nop | | sec | clc | | | | ret | rti | | | | | | |
| 1- | ld a, N | ld b, N | ld c, N | ld d, N | ld e, N | ld f, N | ld g, N | ld h, N | ld ab, NN | ld cd, NN | ld ef, NN | ld gh, NN | ld sp, NN | ld R, (RR) | ld R1, R2 | ld R, (RR+N) |
| 2- | ld NN, a | ld NN, b | ld NN, c | ld NN, d | ld NN, e | ld NN, f | ld NN, g | ld NN, h | ld (RR), R | ld (RR), N | | | | | | ld (RR+N), R |
| 3- | inc a | inc b | inc c | inc d | inc e | inc f | inc g | inc h | inc ab | inc cd | inc ef | inc gh | inc sp | inc (RR) | inc (NN) | |
| 4- | dec a | dec b | dec c | dec d | dec e | dec f | dec g | dec h | dec ab | dec cd | dec ef | dec gh | dec sp | dec (RR) | dec (NN) | |
| 5- | add a, N | add b, N | add c, N | add d, N | add e, N | add f, N | add g, N | add h, N | add ab, N | add cd, N | add ef, N | add gh, N | add sp, N | | | add R1, R2 |
| 6- | sub a, N | sub b, N | sub c, N | sub d, N | sub e, N | sub f, N | sub g, N | sub h, N | sub ab, N | sub cd, N | sub ef, N | sub gh, N | sub sp, N | | | sub R1, R2 |
| 7- | cmp a, N | cmp b, N | cmp c, N | cmp d, N | cmp e, N | cmp f, N | cmp g, N | cmp h, N | cmp ab, N | cmp cd, N | cmp ef, N | cmp gh, N | cmp sp, N | | | cmp R1, R2 |
| 8- | and a, N | and b, N | and c, N | and d, N | and e, N | and f, N | and g, N | and h, N | and ab, N | and cd, N | and ef, N | and gh, N | and sp, N | | | and R1, R2 |
| 9- | test a, N | test b, N | test c, N | test d, N | test e, N | test f, N | test g, N | test h, N | test ab, N | test cd, N | test ef, N | test gh, N | test sp, N | | | test R1, R2 |
| A- | | | | | | | | | | | | | | | | |
| B- | | | | | | | | | | | | | | | | |
| C- | | | lsr R, S | lsr R1, R2 | | | | | | | | | | | | |
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
