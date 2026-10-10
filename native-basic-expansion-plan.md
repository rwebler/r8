# Native BASIC expansion plan

Status: implemented.

## Goal and compatibility boundary

Make room for multidimensional arrays, `DEF`/`FN`, `PRINT AT`, and, if the
assembled budget permits, `LINE`. Keep the stock RX-82 behavior on `main`:
RAM at `0000`–`BFFF`, stock firmware at `C000`–`C0FF`, device addresses at
`FF00` and above, and the reset vector at `FFFE`. Do not change the stock
firmware, CPU address decoding, device registers, or normal `run`/`mon`
machines. The BASIC module remains optional and entered at `C100`.

The stock firmware bytes throughout `C100`–`FEFF` are zero. The `basic` branch's
module installer already permits an image through `FEFF`. The current BASIC
image is 11,767 bytes; the full module window is 15,872 bytes. Thus the layout
offers 4,105 bytes before relocation code and new features. Recheck the stock
image and branch relationship before merging if `main` has changed.

## Proposed BASIC layout

| Addresses | BASIC use | Change |
| --- | --- | --- |
| `0100`–`01FF` | General RAM | Keep available for small user transfers. |
| `0200`–`0FFF` | Existing input, variables, frames, descriptors, scratch | Keep existing allocations; reserve `0374`–`03A7` for function pointers. |
| `1000`–`7FFF` | Packed program | Reduce capacity from 32 KiB to 28 KiB. Last legal zero pair starts at `7FFE`. |
| `8000`–`8FFF` | Stable strings growing up; temporaries growing down | Move the complete 4 KiB pool from `EF00`–`FEFF`. Keep its one-based handles and compaction scheme. |
| `9000`–`9FFF` | Shared integer and string array allocation | Keep the 4 KiB pool; multidimensional metadata consumes bytes from it. |
| `A000`–`BFFF` | Long input spill and CPU stack | Keep current boundaries and stack depth. |
| `C100`–`FEFF` | Optional BASIC ROM | Permit the full existing extension window, ending before I/O at `FF00`. |

This changes BASIC's documented program limit and removes the extra
`EF00`–`FEFF` RAM window from BASIC `BLOAD`/`BSAVE`. Binary transfers remain
valid within `0000`–`BFFF`, including the new string pool; writing over BASIC
storage can still disrupt BASIC. `PEEK`/`POKE` at `EF00`–`FEFF` now see ROM
semantics in a BASIC machine. Text `SAVE`/`LOAD` formats and the stock machine's
memory map stay the same. Update both interpreters and their manuals together.

## Assembly gate after every step

Finish **each numbered step** with a real assembly of the complete ROM:

```sh
cargo run --offline -p rx82 -- asm crates/rx82/sys/basic_rom.asm
wc -c crates/rx82/sys/basic_rom.bin
```

Record the byte count, exclusive end (`C100 + count`), and remaining bytes
(`15872 - count`) in the step's implementation notes. Require a size at most
15,872 bytes and an exclusive end at most `FF00`. Confirm the generated image
matches the source, update the screen hook constants in `native.rs` from the
assembled labels, and resolve any short-branch displacement error before
starting the next step. Never use source-line counts as a capacity estimate.
Keep the source and generated image in the same change. Run the relevant
behavior checks for each step and the workspace suite after the final step.

## 1. Establish the expanded ROM and RAM boundary

Change the native program editor and long-line staging limits from `8FFF` /
`8FFE` to `7FFF` / `7FFE`. Change the host tokenized `Program` capacity from
`0x8000` to `0x7000` bytes, including its admission check for staged long
lines, and its RAM image synchronization to stop at `7FFF`. Preserve the packed
record format and atomic rejection of an oversized edit or load.

Move the string pool's stable base to `8000`, temporary top to `9000`, and
one-based handle conversion base to `7FFF`. Audit each pool constant in
`basic_rom.asm` individually: `FF00` is also the console address, and `9000`
is also the array base. Remove the extra `EF00`–`FEFF` RAM device from the
native BASIC machine so the optional ROM can occupy that space. Update the
reference runner's binary range policy and the BASIC help text. Preserve the
4 KiB string budget and the behavior of allocation, compaction, assignment,
`LEN`, and string arrays.

Review tests and documentation that inspect the old pool address, program
capacity, or `EF00` binary transfers. Check full program and string-pool
boundaries, failed `LOAD`/`BLOAD`, and the stock firmware/reset vector.
**Assemble and record capacity before step 2.**

## 2. Add multidimensional arrays

Support one, two, and three dimensions for integer and string arrays. `DIM
A(2,3)` allocates 12 elements with inclusive, zero-based bounds; indexing is
row-major. The product of dimension lengths must be positive and at most
2,048. All array allocations, including metadata, must fit in `9000`–`9FFF`.
`LEN(array)` returns the product of its dimensions. Existing one-dimensional
arrays keep their current descriptor and element layout.

Keep each four-byte descriptor's absolute element base. Encode rank in unused
high bits of its current upper-bound word and total element count minus one
in the low 11 bits; rank one uses the old encoding. For rank two or three,
place the dimension lengths as little-endian words immediately before the
first element and point the descriptor base at that first element. This costs
four or six bytes per multidimensional array from the existing array pool,
with no new fixed RAM region. Bound-check every subscript before calculating
its flattened offset; reject arithmetic overflow and a wrong subscript count.
Mask the rank bits wherever the old upper bound is read, especially `LEN` and
string-pool reference rewriting.

Extend `DIM`, numeric and string location parsing, `INPUT`/`READ` targets, and
the reference interpreter in the same step. Check first/last elements,
out-of-range indices in every dimension, pool exhaustion, reset, and string
compaction with multidimensional references. **Assemble and record capacity
before step 3.**

## 3. Add `DEF` and `FN`

Use numbered, standalone definitions such as `DEF FNA(X)=X*X+1` and numeric
calls such as `FNA(5)`. Support 26 function names (`FNA`–`FNZ`), one signed
integer argument, and the existing checked integer expression rules. Reserve
token `B6` for `DEF`; leave `AD` for `LINE` and `AE` for the previously
reserved `SOUND`. Function names can remain ordinary letter tokens. Update
both token codecs and LIST/SAVE spacing without changing existing token codes.

Store 26 two-byte pointers at `0374`–`03A7`. On `RUN`, scan the validated
program chain, validate standalone definitions, reject duplicates, and build
the directory before executing user statements; this permits forward calls.
Traverse token payloads with the existing token-aware line walker so embedded
zero bytes, `DATA`, and `REM` cannot be mistaken for definitions or line ends.
Clear or rebuild the directory after edits, `NEW`, and successful `LOAD`, so
no pointer survives a moved program record. Reject direct-mode definitions,
undefined names, malformed signatures, and string arguments. Calls evaluate
their argument first, then evaluate the stored body with a stack-scoped formal
parameter and saved source cursor. Nested calls must respect the existing
`A000` stack floor; errors must discard bindings without changing global
scalar variables.

Implement equivalent definition and call behavior in the reference runner.
Check forward definitions, nested calls, edit invalidation, error recovery,
and signed overflow. **Assemble and record capacity before step 4.**

## 4. Add `PRINT AT`

Use `PRINT AT row,column; items`, with row `0`–`23` and column `0`–`39`.
Reserve token `B7` for `AT`. Require a screen console route, parse and
validate both coordinates and the separator before changing the cursor, then
reuse ordinary PRINT formatting. A trailing semicolon retains the cursor;
ordinary PRINT newline and wrapping behavior remains in force. In picture
mode, write to the retained text page as the current screen PRINT path does.

Set the existing screen cursor byte offset (`00F0`–`00F1`) and column
(`00F2`); do not add hardware registers or consume the array/string pools.
Implement the same behavior in the reference screen console. Check corners,
wrapping, scrolling, picture-mode retention, invalid coordinates, and serial
route errors. **Assemble and record capacity before step 5.**

## 5. Add `LINE` if the assembled budget permits

Use token `AD`, already reserved for `LINE` in `video-sound-deferred.md`.
Implement `LINE x1,y1,x2,y2` for picture mode, with `x` in `0`–`159` and
`y` in `0`–`95`. Parse and validate all four endpoints before any pixel write.
Draw both endpoints with signed Bresenham through the existing `FF36`,
`FF37`, and `FF3A` PLOT ports. Preserve the VRAM pointer and palette. Use
the same algorithm in the reference runner and leave `AE` reserved for SOUND.

Before insertion, compare the remaining measured space with an isolated
assembled LINE candidate, including token, dispatch, HELP, and branch growth.
If it does not fit, keep steps 1–4 complete and report the exact shortfall;
do not change `main`'s hardware map or silently reduce string, array, or stack
capacity. Check horizontal, vertical, diagonal, reversed, single-pixel,
invalid, and off-mode lines. **Assemble and record the final capacity.**

## 6. Final integration

Update the native and reference manuals, memory map, limits, examples, and
historical capacity notes with the actual final image size. Confirm the ROM
source/image match, the stock firmware bytes and reset vector remain intact,
and normal machines still expose `8000`–`8FFF` as ordinary RAM. Check every
fixed screen hook address against assembly labels. **Assemble once more after
the final ROM edit and record the final count and free bytes.**

## Implementation measurements

Each count below comes from assembling the complete ROM. End addresses are
exclusive; free space is measured against the 15,872-byte `C100`–`FEFF` window.

| Step | ROM bytes | Exclusive end | Free bytes |
| --- | ---: | ---: | ---: |
| 1. RAM and ROM boundary | 11,744 | `EEE0` | 4,128 |
| 2. Multidimensional arrays | 12,285 | `F0FD` | 3,587 |
| 3. `DEF` and `FN` | 12,861 | `F33D` | 3,011 |
| 4. `PRINT AT` | 13,070 | `F40E` | 2,802 |
| 5. `LINE` candidate | 13,453 | `F58D` | 2,419 |
| 6. Final ROM and help text | 13,499 | `F5BB` | 2,373 |

The `LINE` candidate was assembled separately before inclusion. The checked-in
ROM source and binary use the final measurement. The stock firmware, hardware
addresses, and reset vector are unchanged.
