# Deferred video and sound designs

This is the broader 2026-10-09 design retained for future assessment. It is not
the active release contract; see `video-sound-plan.md`, `video-sound-spec.md`,
and `screen-console-plan.md`. Its serial-only and deferred cursor/scrolling
statements are superseded by the screen console implementation.

# RX-82 video and sound

Status: broader contract frozen on 2026-10-09, subsequently narrowed by the
accepted restrained milestone in `video-sound-plan.md`. That plan is authoritative
for current implementation scope and explicit overrides: PRINT stays serial;
LINE, SOUND, cursor/scrolling, external video interrupts and the BASIC frame
handler are deferred, possibly to a future architecture. FF3B is reserved
(read zero, ignore writes) in this milestone. Retained device details below
continue to apply; deferred designs are future reference, not release requirements.
The ROM assessment now records a measured 355-byte native addition for the
reduced scope, leaving 444 bytes free; its broader-scope estimate is historical.
The live milestone still includes a display window, speaker output, and headless
operation.

No new opcodes or main-RAM layout changes. Console FF00–FF02, file FF10–FF14,
and random FF20–FF23 retain their functions. Video owns its RAM; it is never
mapped over program or string memory. BASIC ROM begins at C100 and must end
before EF00. Its original 11,349-byte image ends at ED54, leaving 427 bytes;
the former E259 endpoint was obsolete.

## Video registers: FF30–FF3B

Each accepted bus transaction has its side effects exactly once. An address
held for several CPU ticks is one transaction; consecutive transactions to the
same port remain distinct. Reads of unassigned FF3C–FF3F return zero and writes
are ignored. All register values reset to zero except those specified below.

| Port | Read | Write |
| :--- | :--- | :--- |
| FF30 | bit 7: vblank; bit 0: active mode; other bits zero | 0: text, 1: picture; other values ignored |
| FF31 | VRAM pointer low | VRAM pointer low |
| FF32 | VRAM pointer high | VRAM pointer high |
| FF33 | active-page byte, then increment pointer | active-page byte, then increment pointer |
| FF34 | ink, 0–15 | ink, low nibble |
| FF35 | paper, 0–15 | paper, low nibble |
| FF36 | X latch | X latch, all eight bits |
| FF37 | Y latch | Y latch, all eight bits |
| FF38 | picture slot 2 color, 0–15 | slot 2 color, low nibble |
| FF39 | picture slot 3 color, 0–15 | slot 3 color, low nibble |
| FF3A | zero | plot low two bits as a picture slot at X,Y |
| FF3B | bit 0: interrupt enabled; bit 7: event pending | bit 0: interrupt enable; bit 7: write-one-to-clear pending |

Readback of mode and color registers lets BASIC honor direct POKE changes.
Writing a valid mode resets the VRAM pointer, including writing the current
mode. It preserves both pages, palette, X,Y, and BASIC's text cursor. An invalid
mode write changes nothing. COLOR does not draw or change stored attributes.

The VRAM pointer is a 16-bit unsigned counter, wrapping at 65536. FF33 outside
the active page reads zero or ignores a write, and still increments. Reads and
writes to pointer halves take effect immediately and preserve the other half.

FF3A does nothing in text mode or when X >= 160 or Y >= 96. It preserves the
other three pixels in the addressed byte, does not change the VRAM pointer,
and ignores its upper six bits. This command is the only plotting side effect;
writing X, Y, ink, or paper does not plot.

Reset state: text mode, pointer/X/Y zero, ink 0, paper 7, slot 2 color 2, slot 3
color 4. Thus the picture palette initially maps slots 0–3 to colors 7,0,2,4.
Text RAM contains 960 `(20,70)` pairs and picture RAM contains 3840 zero bytes.
The native BASIC cursor is initialized to column 0, row 0 at BASIC startup.

### Text page

Mode 0 displays 40 columns by 24 rows of 8-by-8 glyphs: 320 by 192 pixels.
Cell `(x,y)` occupies bytes `2*(40*y+x)` and the following byte, character then
attribute. Attribute low nibble is ink and high nibble is paper. Page size is
1920 bytes, offsets 0000–077F. Existing cell colors do not change with COLOR.

ASCII 20–7E uses an 8-by-8 controller font. Use an original or suitably licensed
font asset with its license recorded; the exact ASCII artwork is an asset
choice, not a software-visible register behavior. Unsupported character codes
render blank, including 7F and A0–FF. Stored VRAM bytes are never rewritten by
the renderer. BASIC PRINT's control handling is specified separately below.

The block set 80–9F is fixed by these bitmap rules (row byte bit 7 is leftmost):

- For 80–8F, low-nibble bits select filled quadrants: bit 0 top-left, bit 1
  top-right, bit 2 bottom-left, bit 3 bottom-right. Each quadrant is 4 by 4.
  For example 80 is blank, 81 is `F0 F0 F0 F0 00 00 00 00`, and 8F is solid.
- The remaining glyphs have these eight row bytes:

| Code | Rows, hexadecimal |
| :--- | :--- |
| 90 | 18 18 18 18 18 18 18 18 |
| 91 | 00 00 00 FF FF 00 00 00 |
| 92 | 18 18 18 FF FF 18 18 18 |
| 93 | 00 00 00 1F 1F 18 18 18 |
| 94 | 00 00 00 F8 F8 18 18 18 |
| 95 | 18 18 18 1F 1F 00 00 00 |
| 96 | 18 18 18 F8 F8 00 00 00 |
| 97 | 18 18 18 1F 1F 18 18 18 |
| 98 | 18 18 18 F8 F8 18 18 18 |
| 99 | 00 00 00 FF FF 18 18 18 |
| 9A | 18 18 18 FF FF 00 00 00 |
| 9B | AA 55 AA 55 AA 55 AA 55 |
| 9C | FF 81 81 81 81 81 81 FF |
| 9D | 7E 42 42 42 4A 42 42 7E |
| 9E | 7E 46 4A 52 62 42 42 7E |
| 9F | FF 00 FF 00 FF 00 FF 00 |

### Picture page and palette

Mode 1 displays 160 by 96 pixels with four slots, two bits per pixel, in 3840
bytes (offsets 0000–0EFF). Byte offset is `40*y + floor(x/4)`. From left to
right, pixel slots occupy bits 7–6, 5–4, 3–2, 1–0. Slot 0 uses paper, slot 1
uses ink, slots 2 and 3 use FF38 and FF39. Palette changes recolor existing
picture pixels. BASIC PLOT always writes slot 1; direct FF3A writes or uploads
can use all four slots. A paper clear is 3840 zero bytes.

The fixed global palette is below. Indices retain the original spec's color
ordering, so paper 7 is yellow. Half-bright values are explicitly rounded down.
Index 8 intentionally duplicates black.

| Index | Name | RGB hexadecimal |
| ---: | :--- | :--- |
| 0 | black | 000000 |
| 1 | white | FFFFFF |
| 2 | red | FF0000 |
| 3 | cyan | 00FFFF |
| 4 | purple | FF00FF |
| 5 | green | 00FF00 |
| 6 | blue | 0000FF |
| 7 | yellow | FFFF00 |
| 8 | half-bright black | 000000 |
| 9 | half-bright white | 7F7F7F |
| 10 | half-bright red | 7F0000 |
| 11 | half-bright cyan | 007F7F |
| 12 | half-bright purple | 7F007F |
| 13 | half-bright green | 007F00 |
| 14 | half-bright blue | 00007F |
| 15 | half-bright yellow | 7F7F00 |

Display picture pixels at 2 by 2 on the same 320-by-192 surface as text.
Window resizing uses integer nearest-neighbor scaling with letterboxing.
There are no sprites, blits, hardware scrolling, or raster splits.

## Timing and external traps

The emulated system frequency is 4,000,000 ticks per second. On each tick add
60 to a frame-phase accumulator initially zero; subtract 4,000,000 on wrap.
Vblank is true when phase >= 3,600,000 (the final 10% of a frame). This is the
chosen timing model, not an assertion of exact historical scan timing. Use
wide arithmetic independent of the existing wrapping 16-bit debug counter.

Entering vblank sets one pending event even when interrupt delivery is disabled.
Further entries coalesce while pending. FF30/FF3B reads never acknowledge it.
FF3B bit 7 clears it; bit 0 always replaces the enable state on every write.
For example 81 enables and clears, 01 enables without clearing, and 80 disables
and clears. Reset disables delivery and clears pending and phase.

Reserve trap code 01, vector bytes 0002 (low) and 0003 (high), for video. This
vector is unused by stock firmware; the software trap instruction remains
unchanged. The emulator needs an external request path because the current
bus does not have one. Native BASIC installs its handler before writing FF3B.
Other guests must initialize a valid vector before enabling delivery.

Accept a pending enabled video event at an instruction boundary before fetching
the next opcode, only if the CPU is running and outside a trap handler. Consume
the pending event on acceptance. Use the existing trap entry and stack format,
with return PC identifying the next instruction. RTI returns from the trap
(the assembler instruction is RTI, not RETTRAP). Track trap depth to defer
external video delivery throughout software and hardware trap handlers. An
event arriving in a handler can be accepted after the outermost RTI. Normal
software TRAP instructions retain their current nesting semantics.

The BASIC handler preserves all general registers; RTI restores flags. It
increments a 16-bit frame counter modulo 65536; this counts accepted events,
not coalesced frames. Its RAM location must be allocated in the implementation
scratch audit without moving any existing region. No BASIC ON FRAME command
or automatic duration facility is introduced.

CPU reset disables and clears video interrupt delivery and trap depth, but
preserves device RAM, palette, sound registers and frame phase. Cold device
reset restores all device defaults. HALT is not woken by video; live device time
can continue until the runner exits, with pending events coalesced. A debugger
pause freezes device time and suppresses interrupt delivery. Monitor PEEK still
performs register read side effects, but its synthetic service ticks must not
advance video/audio time or accept interrupts while paused.

Real-time native execution is paced to 4 MHz; devices keep advancing while
terminal input is awaited. Host BASIC uses the same device clock advanced by
its runner, and exposes frame events to its harness; it has no emulated CPU
trap stack and does not promise cycle-exact BASIC timing parity. Headless
harnesses advance time explicitly. Turbo advances emulated time faster, drops
stale display frames, and silences live audio rather than accumulating a queue.

## Sound: FF40–FF47

Three independent square channels and one independent noise channel share a
1,789,773 Hz chip clock. There is no envelope, DMA, or AY register compatibility
promise. Clock conversion from system ticks uses an integer remainder so
chunking elapsed time does not affect the generated state.

| Port | Read and write |
| :--- | :--- |
| FF40 | tone 0 period low |
| FF41 | tone 0 period high, low nibble only |
| FF42 | tone 1 period low |
| FF43 | tone 1 period high, low nibble only |
| FF44 | tone 2 period low |
| FF45 | tone 2 period high, low nibble only |
| FF46 | noise period bits 0–4; bits 5–7 independently mute tones 0–2 when set |
| FF47 | volume 0 bits 0–1; volume 1 bits 2–3; volume 2 bits 4–5; noise bits 6–7 |

Every read returns the stored masked value; reserved high bits read zero.
Each write applies immediately; a low/high period write is not an atomic pair.
All registers reset to zero. Noise mute does not follow tone mute bits.

For nonzero period P, each tone toggles after 8*P chip clocks, giving frequency
`1789773/(16*P)`. Period zero silences and holds its oscillator at positive
polarity with counter zero. Changing either period byte resets that channel's
counter and polarity. Volume/mute changes preserve oscillator phase; muted
nonzero oscillators continue advancing.

Noise uses a 17-bit right-shifting LFSR, seed 1FFFF. At each shift compute
`feedback = (state bit 0) XOR (state bit 3)`, then
`state = (state >> 1) | (feedback << 16)`. Output is the current bit 0, mapped
zero to negative and one to positive. Shift once per `16*noise_period` chip
clocks. Noise period zero silences and holds the reset seed/counter. A change
to the low five period bits resets seed/counter; changing only tone mute bits
does not. This precise recurrence defines the noise sequence.

Volumes 0–3 map to amplitudes 0, 1/3, 2/3, 1. Sum the four signed channels and
divide by four for fixed headroom. Device synthesis offers deterministic mono
48,000 Hz samples, duplicable to stereo by the backend. At each sample boundary
use the current chip output after processing all chip edges through that time.
Host resampling may adapt to hardware; device tests use the canonical samples.

`SOUND 0,428,3` approximates middle C (about 261.36 Hz). The old period 478 was
inconsistent with the formula. Duration is controlled by subsequent SOUND
commands, guest loops, or guest frame counters. Console clicks are excluded:
there is no existing click mixer, and this contract defines no click waveform.

## BASIC behavior in both runners

Statements work at the prompt, in programs, and through existing control flow.
Parse the full statement and validate arguments before device writes. Numeric
arguments use existing checked integer expressions; a range or mode violation
reports `? ILLEGAL QUANTITY`. Normal expression side effects such as RND remain.
Syntax errors retain the existing syntax diagnostic.

- `SCREEN mode`: accept 0 or 1, write FF30. Preserve both pages and text cursor.
- `CLS`: clear the active page. Text is spaces with current ink/paper attributes
  and cursor 0,0; picture is slot zero. Set pointer low then high to zero and
  stream exactly the active page size through FF33. Palette is preserved.
- `COLOR ink,paper`: both 0–15; write FF34 then FF35.
- `PLOT x,y`: mode 1 only, x 0–159 and y 0–95. Write X to FF36, Y to FF37,
  then 1 to FF3A. Leave ink, paper, and VRAM pointer unchanged.
- `LINE x1,y1,x2,y2`: same range/mode checks for both endpoints before plotting.
  Draw both endpoints with the PLOT sequence for every point. Use signed
  Bresenham: dx=abs(x2-x1), dy=-abs(y2-y1), sx/sy are +1 or -1 toward the end,
  err=dx+dy; plot, stop at endpoint; e2=2*err; if e2>=dy, err+=dy and x+=sx;
  if e2<=dx, err+=dx and y+=sy. Both tests use the original e2. No clipping.
- `SOUND channel,period,volume`: channel 0–3, volume 0–3; period 0–4095 for
  tones and 0–31 for noise. For tones write low then high period, then read,
  modify and write FF47 preserving other volumes. For noise first read,
  modify and write FF46 preserving mute bits, then update FF47. SOUND does not
  clear a tone's explicit hardware mute bit. POKE changes remain observable.

Reserve keyword codes A9–AE in order SCREEN, CLS, COLOR, PLOT, LINE, SOUND;
existing token values remain unchanged. PLAY is outside scope.

PRINT alone follows the mode: text mode writes text VRAM; picture mode writes
the serial console and does not modify either page. Prompts, INPUT prompts,
diagnostics, LIST, HELP, monitor output, and file serialization keep their
existing destinations. Returning to SCREEN 0 reveals its retained contents.
A caption intended for that page must be printed while SCREEN 0 is active.

Text PRINT handles LF by setting column zero and advancing a row; CR sets
column zero. TAB writes spaces to the next multiple-of-eight column, wrapping
normally. Other control bytes below 20 are ignored. Printable and block bytes
write a character plus current attribute at the cursor, then advance it.
Advancing beyond column 39 wraps immediately. Advancing beyond row 23 scrolls
rows 1–23 upward and clears row 23 with current attributes. The cursor stays
on row 23. PRINT's existing numeric/string formatting, comma tabs, semicolon
suppression and implicit newline are retained. CLS resets this software cursor;
SCREEN and raw VRAM access do not.

Native text scrolling and line drawing execute in ROM using the ports. Device
hardware contains no BASIC-aware acceleration. A complete picture upload must
not allocate a BASIC string: READ numeric DATA or stream file-port bytes to
FF33. BLOAD remains a RAM-only command and cannot target a device stream.

## Live frontend and acceptance

The window and audio path are included, with optional headless operation.
Terminal input must not block window events or device clocks. Audio queues are
bounded, underrun emits silence, and no audio hardware permits video operation
with a clear diagnostic. EOF/QUIT or closing the window ends the live session
and releases audio. Backend selection and font artwork remain implementation
choices within this contract.

Required checks include text `PRINT "AB"` producing `41 70 42 70` with ink 0,
paper 7; picture PLOT 0,0 setting byte 0's top two bits to 01; stable full-page
uploads; preserved pages across SCREEN; exact port side effects; tone/noise
sample fixtures; vblank status and safe trap acceptance/return; and identical
normalized device transaction traces for deterministic BASIC operations in both
runners. Compare timed reads at controlled phases, not at matching interpreter
instruction counts. Generated BASIC ROM must match source and fit below EF00.
