# RX-82 video and sound active specification

The restrained milestone in `video-sound-plan.md` is authoritative. This file
records the active device and BASIC contract. Earlier designs for text PRINT,
LINE, SOUND, and external interrupts are in `video-sound-deferred.md`.

Video and sound own their memory and registers. The 4 MHz system clock is
independent of the 16-bit debug cycle counter. CPU reset does not reset the
devices; cold device construction restores the defaults below.

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
| FF3B | zero | ignored (reserved) |

Readback of mode and color registers lets BASIC honor direct POKE changes.
Writing a valid mode resets the VRAM pointer, including writing the current
mode. It preserves both pages, palette, X,Y. An invalid
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
There is no hardware or BASIC text cursor in this release.

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

## Timing and polling

The emulated system frequency is 4,000,000 ticks per second. On each tick add
60 to a frame-phase accumulator initially zero; subtract 4,000,000 on wrap.
Vblank is true when phase >= 3,600,000 (the final 10% of a frame). FF30 bit 7
reports this state without an acknowledgment side effect. FF3B is inert and
there is no video interrupt or CPU trap change. Debugger inspection performs
register reads without advancing device time. Headless harnesses can advance
device time explicitly. Live runners continue the device clock while terminal
input is awaited. Turbo drops stale frames and silences live audio.
Headless reference BASIC advances 1,000 device ticks after each statement;
harnesses can advance additional time explicitly.

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

Period 428 approximates middle C (about 261.36 Hz). Program channel 0 with
`POKE 65344,172`, `POKE 65345,1`, and `POKE 65351,3`. The packed volume
write replaces all four fields. Duration is controlled by later POKE commands
or guest loops. Console clicks are excluded:
there is no existing click mixer, and this contract defines no click waveform.

## BASIC behavior in both runners

Statements work at the prompt, in programs, and through existing control flow.
Parse the full statement and validate all arguments before device writes.
Numeric arguments use checked integer expressions. Range or mode violations
report `? ILLEGAL QUANTITY`; syntax errors retain their existing diagnostic.

- `SCREEN mode`: accept 0 or 1 and write FF30. Preserve both pages.
- `CLS`: set pointer low and high to zero; stream 960 text pairs of space and
  current paper/ink attribute through FF33, or 3840 zero picture bytes.
- `COLOR ink,paper`: both 0–15; write FF34 then FF35.
- `PLOT x,y`: picture mode only, x 0–159 and y 0–95; write FF36, FF37, then
  slot 1 to FF3A. Preserve the palette and VRAM pointer.

Keyword codes A9–AC are SCREEN, CLS, COLOR, PLOT. AD and AE remain unused.
PRINT, prompts, INPUT, diagnostics, LIST, and HELP remain on the serial
console in both modes. Text screens use raw VRAM character/attribute pairs.
Pictures may be uploaded by READ/DATA or by file-port bytes streamed into
FF33. BLOAD remains RAM-only.

## Live frontend and validation

The `live` Cargo feature enables SDL2 video/audio through `basic --live`, for
native or reference BASIC. Window resizing uses integer nearest-neighbor
scaling and letterboxing. Video works if audio hardware is unavailable, with
a diagnostic. Audio queues are bounded; underruns play silence. Closing the
window, terminal EOF, or QUIT ends the session. A debugger pause freezes
device time. Live turbo drops stale frames and silences audio.

Check exact bus side effects, page retention, four pixel slots, block glyphs,
full uploads, ROM/source equality and capacity, tone/noise fixtures, and
vblank before/during/after the final 10% interval. Compare native and reference
device transactions at controlled phases; CPU instruction counts need not match.
