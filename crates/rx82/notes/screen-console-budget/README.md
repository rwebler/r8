# Native screen console measurement

The isolated output prototype was integrated into `sys/basic_rom.asm` and
extended with a guest line editor, backspace, INPUT mode restoration, and a
guest break recovery entry. Run `python3 crates/rx82/notes/screen-console-budget/verify.py`
from the repository root to assemble the production source, compare it with
the bundled binary, check its size, and run native video sessions.

| Item | Bytes |
| --- | ---: |
| Pre-console BASIC ROM | 11,332 |
| Integrated screen console ROM | 11,774 |
| Growth | 442 |
| Free before EF00 | 2 |

The exclusive end is `EEFE`. The firmware, program, stack, arrays, and string
pool boundaries are unchanged. The shortened native HELP text remains a useful
command summary. The `routines.asm` file records the isolated candidate; the
production source is authoritative.

The guest uses F0–F2 for text cursor position, F3 for route selection, F4–F8
for raw-line replay, F9 for input/preload state, and FA–FC for saved INPUT
picture mode and VRAM pointer. These bytes were unused in native BASIC. The
editor uses A400–A7FF for up to 1024 bytes of raw input before replaying them
through the existing token reader. This protects the prompt and avoids stale
tokens after Backspace. A 1025th byte rejects and drains the line with
`LINE TOO LONG`, leaving no partial command. Screen preload bypasses editing and suppresses output
until RUN begins; file input and serial sessions retain their prior paths.

The frontend draws a cursor overlay from guest F0–F2 and F9 while input waits;
VRAM is unchanged by the overlay. It defers presenting frames while the guest
is inside SC_OUTPUT through SC_CELL, preventing transient FF30 page selection
from appearing as an intentional SCREEN change. The deferral ends when the
emitter returns; device and audio time continue. Ctrl-C enters SC_BREAK at an
instruction boundary, aborts the file stream, clears console state, and jumps
to the common prompt. Tests verify the assembly addresses used by the frontend.

The agreed 1024-byte editable screen input limit is narrower than the longest
accepted streamed serial program line. Automated tests
exercise ROM/source equality, editor backspace, picture INPUT restoration,
break recovery, silent preload, output, scrolling, serial routing, files, and
reference text attributes. The live keyboard queue is bounded; a full queue
rejects the pending screen line once Enter arrives. Physical keyboard, audio, and normal/turbo frame
validation remain outstanding. A synthetic SDL event injection test crashed
inside this machine's SDL2 compatibility library; the event decoder is tested
without injecting synthetic SDL events.
