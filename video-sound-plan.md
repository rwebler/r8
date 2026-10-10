# RX-82 video and sound implementation plan

Status: video/sound steps 2–6 implemented on 2026-10-09. The later
`screen-console-plan.md` supersedes the serial-only BASIC output and absent
cursor statements below. The current native BASIC ROM is 11,774 bytes
(exclusive end EEFE); physical window/keyboard/audio validation remains open.
The 11,332-byte ROM and 444-byte headroom cited in the historical measurements
below describe the earlier video/sound milestone.
This plan supersedes the broader implementation scope in the earlier frozen
`video-sound-spec.md`. Retained device behavior follows that spec, subject to
these explicit overrides. Deferred features are future enhancements, possibly
for a future architecture, and are not prerequisites for this release.

## Intended implementation

| Area | Included |
| :--- | :--- |
| Display | Working window; 40-by-24 text and 160-by-96 picture modes |
| Video memory | Independent retained pages, palette, raw VRAM access, pixel command |
| BASIC | SCREEN, CLS, COLOR, PLOT in both interpreters |
| Text output | PRINT stays on the serial console in both modes |
| Sound | Three tone channels and one noise channel, register access through PEEK/POKE, speaker output |
| Timing | 60 Hz refresh and readable vblank status; polling only |
| Automation | Headless device operation and deterministic clock advancement |

Programs construct text screens by writing character/attribute pairs to VRAM.
Picture adventures can show an illustration in the window and captions in the
terminal. Line drawings can use BASIC loops calling PLOT or uploaded picture
bytes. Sound is controlled by POKE, with PEEK for register readback.

## Contract retained and overridden

### Video

Retain FF30–FF3A and their frozen semantics: mode/status readback, a 16-bit
VRAM pointer, auto-incrementing data access, ink/paper, X/Y, two extra palette
registers, and the explicit pixel command. Retain both page sizes, packed
pixel order, fixed palette, block glyph definitions, font requirements, reset
values, invalid-access behavior, and one side effect per bus transaction.

SCREEN changes the active page and resets the hardware VRAM pointer while
preserving both pages and palette. CLS fills the active text page with spaces
and current attributes, or the picture page with slot zero. There is no BASIC
text cursor to initialize, preserve, or reset in this release. Raw VRAM writes
are the text-page interface; the device does not interpret control characters
or scroll text.

### BASIC

Implement only these four new statements, with the frozen argument rules:

- `SCREEN mode`: mode 0 or 1; write FF30.
- `CLS`: select pointer zero and stream the active page's clear bytes via FF33.
- `COLOR ink,paper`: values 0–15; write FF34 then FF35.
- `PLOT x,y`: picture mode only, x 0–159 and y 0–95; write FF36, FF37, then
  slot 1 to FF3A. Preserve palette and VRAM pointer.

Reserve A9–AC for SCREEN, CLS, COLOR, PLOT. Leave AD/AE unused for possible
future LINE/SOUND support; do not add their keywords, parsers, or dispatch.
All four statements work in direct mode, programs, and existing control flow.
Parse and validate complete statements before writes. Range/mode errors use
ILLEGAL QUANTITY; syntax and expression errors retain existing behavior.

PRINT uses the serial console in both modes with its existing formatting.
Prompts, INPUT, diagnostics, listings, HELP, and file serialization keep their
existing destinations. No output-routing flag or video character emitter is
needed. Do not implement deferred commands in the reference interpreter alone.

### Sound

Retain all FF40–FF47 registers, readback, three tones, independent noise,
period/mute/volume rules, reset/phase behavior, deterministic LFSR, fixed mix
headroom, and sample timing from the spec. These live in the device model and
host audio path; there is no new native sound routine or SOUND statement.

For example, with reset mute state, middle C on channel 0 at full volume uses
`POKE 65344,172`, `POKE 65345,1`, and `POKE 65351,3`: period 428, volume 3.
This last write replaces all packed volumes. Examples changing one channel
while others play must preserve the other fields with PEEK and integer
arithmetic. A subsequent zero-volume or zero-period write silences the channel.
No console-click waveform or mixer input is introduced.

### Timing

Retain the 4 MHz emulated clock, fractional 60 Hz frame phase, final-10% vblank
interval, and FF30 bit 7. Polling reads have no acknowledgment side effect.
Keep live pacing, device time while waiting for input, frozen time during a
debugger pause, explicit headless advancement, and bounded turbo behavior
(including silent live audio) from the spec.

FF3B is reserved: reads return zero and writes are ignored, as for FF3C–FF3F.
There is no pending interrupt latch, enable/acknowledgment state, video trap
vector reservation, external CPU interrupt path, trap-depth tracking, BASIC
frame counter, or ROM handler in this release. Existing software traps and
CPU reset semantics remain intact. Cold video reset initializes frame phase;
CPU reset does not reset video phase or device contents.

The host interpreter advances the same device clock through its runner. It
has no cycle-exact timing parity requirement with native BASIC. Compare timed
status reads at explicitly controlled phases in tests.

## Current integration points and ROM constraint

Paths here are relative to `crates/rx82/`.

- `src/system.rs`: device ticking and bus arbitration. Devices must precede
  the system ROM, which covers the device page.
- `src/random.rs`: direct register access and a bus adapter servicing a held
  transaction once; use this pattern for VRAM reads/writes.
- `src/basic.rs`: reference PEEK/POKE routing and statement dispatch.
- `sys/basic_rom.asm` and `src/basic_tokens.rs`: native interpreter and shared
  keyword codec. Reuse existing expression and output helpers.
- `src/native.rs`, `src/main.rs`, `src/monitor.rs`: device attachment, runner
  pacing, terminal interaction, and debugger access.

Completed compaction reduced BASIC ROM from 11,349 to 10,977 bytes. Before
video integration the image ended at EBE0, leaving **799 bytes** before string
RAM at EF00. Preserve
that memory map, existing language behavior, and the current opcode set.

The previous 2,048-byte allocation and 1,249-byte shortfall in
`video-sound-rom-feasibility.md` assessed the broader feature set. They are
historical, not the budget for this milestone. The reduced native candidate has now been assembled and checked: it adds
355 bytes, leaving 444 bytes free. Use a 483-byte working allocation including
128 bytes of integration reserve, leaving 316 bytes unallocated. See the new
measured-fit section in the assessment; this result uses complete routines.

## Implementation sequence

### 1. Establish the reduced native ROM budget — completed

The isolated prototype in `notes/video-sound-budget/` assembles to 11332 bytes
and passes 19 native sessions. The shipped image now includes the candidate.
Actual growth is 355 bytes, with 444 bytes left
before EF00. No further compaction is required for the measured candidate.
The following requirements remain applicable when integrating or revising it.


Assemble complete SCREEN, CLS, COLOR, and PLOT paths using the existing native
calling conventions. Count keyword records, dispatch, shared parsing/range
checks, register access, clear loops, diagnostics, concise HELP additions, and
any branch expansion caused by insertion. Include argument preservation and
error exits; placeholders do not establish feasibility.

Use the available 799 bytes as the total growth limit, including integration
costs. Record actual sizes and remaining headroom in a new section of the ROM
assessment. Further modest behavior-preserving compaction can be considered if
needed. If the four-command implementation cannot fit, report its measured
shortfall before changing the scope or architecture. Extensive restructuring
to accommodate deferred features is no longer part of the plan.

Deliverable: an assembled budget for the four-command milestone below EF00.

### 2. Implement shared video and sound devices

Add `src/video.rs` and `src/sound.rs` and export them from `src/lib.rs`. Keep
register state, rendering, and synthesis independent of host I/O. Provide
direct register methods for reference BASIC and Device adapters for native
execution, with inspectable state for renderers and harnesses.

Video owns both pages, registers, pointer, and frame phase. Render text at
320 by 192 and picture pixels doubled onto that same surface. Use the frozen
palette/block glyphs and an original or suitably licensed ASCII font. Pixel
writes preserve neighboring packed pixels. FF3B remains inert.

Sound owns registers, oscillators, noise state, and fractional clock conversion.
Produce deterministic samples from elapsed emulated time with bounded storage
and no per-tick allocation. Backend callbacks must not affect register behavior.

Attach devices before ROM and route reference PEEK/POKE through the same direct
methods, preserving side effects. Include monitor and assembly runner access.

### 3. Connect frame timing and polling

Advance the device clock independently of the wrapping 16-bit debug counter.
Expose vblank through FF30 without interrupt generation or CPU trap changes.
Support deterministic explicit advancement in headless harnesses.

Ensure debugger register inspection still services reads while paused without
advancing video/audio time. Native and host runners must continue device time
while waiting for terminal input in live mode. Validate the pacing model before
relying on PEEK polling examples.

### 4. Integrate the four BASIC statements

Implement matching host/native semantics from the measured routines. Add only
the four tokens and dispatch entries. Validate before writes and preserve the
statement cursor and existing file-output state. Honor mode/color changes made
through direct POKE by reading the device registers rather than stale shadows.

CLS streams bytes through FF33 in guest ROM; it is not a BASIC-aware device
command. Text clears emit character 20 followed by the current attribute for
960 cells; picture clears emit 3840 zero bytes. No native cursor, text emitter,
scroll routine, Bresenham, sound wrapper, or interrupt handler is added.

Update LIST and text SAVE/LOAD round trips, both HELP outputs, and error recovery.
Reassemble and measure the final image, including any changed branch lengths.

### 5. Integrate the display window and speaker output

Add a live backend behind a Cargo feature and explicit CLI configuration,
retaining headless device operation. Select libraries at implementation time
against platform requirements. Present nearest-neighbor scaled frames and feed
bounded audio buffers independently of terminal input.

Refactor blocking input so window events and device clocks continue while the
guest waits. Specify and implement EOF/QUIT, window close, unavailable audio,
underrun silence, debugger pause, and turbo behavior as in the retained live
contract. Keep device ownership on the emulation thread and transport only the
needed frame/sample data to the backend.

### 6. Add examples and documentation

Provide examples for:

- A text screen constructed with character/attribute POKEs and preserved across
  mode changes; serial PRINT remains visible separately.
- PLOT with BASIC loops drawing a small picture, plus a full 3840-byte READ/DATA
  upload through FF33 while other strings remain live.
- File-port byte streaming into FF33 using the existing protocol. BLOAD remains
  RAM-only and cannot write a device stream.
- Tone/noise control through POKE and packed-register preservation through PEEK.
- Vblank polling without traps, using the existing integer expression syntax.

Update the active spec sections to match these overrides before implementation
is considered complete; move broader behavior to explicitly deferred notes.
Update `crates/rx82/README.md`, examples, port tables, and HELP. Regenerate and
check `sys/basic_rom.bin`. Examples must not depend on deferred capabilities.

## Validation during implementation

- Video registers: held transactions, consecutive same-port operations, pointer
  wrap, invalid accesses, page retention, read side effects, palette changes,
  pixel trigger, reserved FF3B behavior, and neighboring-pixel preservation.
- Text: raw VRAM writes `41 70 42 70` display AB with the specified attributes;
  glyph decoding and CLS are correct. PRINT in either mode leaves both pages
  untouched and produces the expected console output.
- Picture: all four pixel slots, corners, CLS, preserved pages, and full uploads.
- BASIC: valid and invalid modes/colors/coordinates, malformed syntax without
  device writes, loops, IF, GOSUB, direct mode, and saved tokenized programs.
- Sound: raw register masks/readback, packed fields, zero silencing, tone mute,
  period 428 frequency, deterministic noise, and chunk-independent samples.
- Timing: exact long-term 60 Hz accumulation and FF30 before/during/after blank;
  polling has no side effects and no video-generated CPU trap occurs.
- Integration: normalized device transactions for the four statements and raw
  PEEK/POKE agree across runners. Exclude CPU fetches/timestamps; compare timed
  reads at controlled phases. Use independent expected-value assertions too.
- Regressions: serial output, prompts, file serialization, software traps,
  random/file devices, string pool, main memory map, ROM equality and capacity.
- Live checks: both display modes, responsive window/input, audible tones/noise,
  clean exit, pause behavior, and bounded turbo buffers.

Run focused coverage as changes land, then repository checks:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features
cargo test --all-features
```

Validate the headless configuration without display/audio hardware as well.
The original scope amendment changed documentation only; implementation and
runtime checks followed in the work described above.

## Future enhancements, possibly for a future architecture

These are outside implementation, validation, and completion requirements for
this release. Each needs a fresh ROM/architecture assessment before adoption.

| Deferred feature | Possible future work |
| :--- | :--- |
| Text-mode PRINT, cursor handling, scrolling | Mode-aware output, character/control handling, cursor state, ROM scroll/copy routines |
| Native LINE | Checked endpoints and inclusive Bresenham; BASIC PLOT loops/uploads suffice initially |
| SOUND statement | Checked arguments and packed-register updates; PEEK/POKE provide current control |
| External video interrupts | Enable/ack register, pending events, vector assignment, safe instruction-boundary delivery and nesting |
| BASIC frame handler | Vector installation, register preservation, frame counter, and RTI integration |

The broader spec records prior designs for these enhancements, not commitments
to implement them later on the RX-82. A future architecture may offer additional
ROM capacity or different interrupt facilities. PLAY, sprites, hardware scroll,
blitters, raster splits, banked ROM, and memory-map changes also remain outside
this release.

## Completion criteria

Both runners support SCREEN, CLS, COLOR, PLOT and raw video/sound register
access with matching semantics. PRINT stays serial. Retained text/picture pages,
full streamed picture uploads without BASIC string allocation, palette changes,
60 Hz vblank polling, a live display, and speaker output work as specified.
Documentation, examples, deterministic device checks, generated ROM and source
agree. The final native image fits below EF00 with the existing memory map.
Deferred enhancements are not acceptance criteria.
