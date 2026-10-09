# Restrained video ROM capacity prototype

This is an isolated, executable native ROM prototype for SCREEN, CLS, COLOR,
and PLOT. It is not installed in the shipped BASIC ROM and does not implement
the display or sound devices. See the root `video-sound-rom-feasibility.md`.

From the repository root:

```sh
python3 crates/rx82/notes/video-sound-budget/verify.py
```

The script builds the existing Rust libraries offline, materializes a candidate
ROM in a temporary directory, assembles baseline and candidate, checks baseline
source/image equality, and executes 19 native sessions against a video-port
recorder. It prints actual sizes and the temporary artifact directory, including
assembly, binaries, and label maps. It does not replace production source or ROM.

- `build.py` adds four dispatch entries, keyword records, HELP text, and native
  detokenizer spacing, then appends the complete routines in `routines.asm`.
- `measure.rs` uses the real assembler and emits a sorted symbol map.
- `check.rs` boots the candidate with the existing native machine, records
  writes, supplies mode/color readback, and checks commands, exact clear streams,
  invalid arguments, recovery, serial PRINT, LIST, and program control flow.

The recorder does not emulate VRAM or render pixels. Device behavior and host
interpreter parity still belong to implementation validation. The prototype
uses the existing expression parser, comma parser, EOL checks, and common error
recovery; its arguments live in registers and the existing CPU stack. No new
persistent RAM scratch is required. Retain branch-range checks on integration.

At the measured baseline: 10977 bytes existing ROM + 355 bytes growth = 11332
bytes total, exclusive end ED44, leaving 444 bytes before EF00.
