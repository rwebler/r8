# RX-82 screen console implementation plan

Status: implemented in the native ROM and both live runners on 2026-10-09,
with the limits recorded in the [measurement report](crates/rx82/notes/screen-console-budget/README.md).
The 11,774-byte ROM ends at EEFE, leaving two bytes before EF00. Automated guest and decoder checks
pass; physical window/keyboard/audio validation remains outstanding. Screen
editing is capped at 1024 raw bytes as agreed, while serial sessions retain
their longer streamed-line capacity. The historical milestones below record
the original design and should be read with this accepted screen limit.

## Outcome and agreed constraints

Provide a native BASIC computer operated entirely from its video window: boot
banner, prompt, visible cursor, keyboard input, PRINT, INPUT, LIST, HELP, and
diagnostics. Keep serial operation available for terminal use and automation.

Keep the existing R8 instruction set and RX-82 memory map. BASIC remains at
C100 and must end before EF00; EF00–FEFF remains the 4 KiB string pool. Preserve
stock firmware, program storage, arrays, stack boundaries, and device register
semantics. Do not add a host BASIC interpreter shortcut to native execution.

In picture mode, text output updates the retained text page without destroying
the picture. Program completion, STOP, break, and errors return the screen
console to text mode. Merely printing inside a running program does not end
picture mode. Serial sessions do not automatically change the displayed mode.

## Measured starting point

See [prototype report](crates/rx82/notes/screen-console-budget/README.md) and
[ROM assessment](video-sound-rom-feasibility.md).

| Item | Bytes |
| --- | ---: |
| Current native BASIC | 11,332 |
| Prototype output routines and integration | 244 |
| Incidental HELP text saving | -4 |
| Prototype ROM | 11,572 |
| Capacity, C100–EEFF | 11,776 |
| Remaining prototype capacity | 204 |

The prototype supports output, colors, wrapping, scrolling, serial selection,
and prompt mode changes. Eleven native sessions passed against the actual video
device. It uses F0–F3 in existing system RAM. Those addresses are provisional
until the complete runtime-state audit is finished.

The 204-byte remainder is a measured limit, not an allocation estimate for
editing, echo, cursor, or break handling. Each implementation milestone must
assemble the entire ROM and report size, exclusive end, and remaining bytes.

## User-visible contract

| Situation | Behavior |
| --- | --- |
| Window session | Native BASIC, screen console, keyboard focus in window |
| Serial session | Terminal input/output; optional existing video/audio window |
| Boot | Text mode, initialized cursor, BASIC banner and prompt |
| PRINT, LIST, HELP, diagnostics | Selected console; no automatic serial mirror |
| INPUT in picture mode | Reveal text page for interactive entry; restore picture mode after successful input; errors/break stay in text mode |
| SCREEN 0/1 | Preserve both pages and text cursor; retain existing hardware pointer reset |
| CLS in text mode | Clear using current colors; move cursor to top left |
| CLS in picture mode | Clear picture; preserve text cursor/page |
| COLOR | Subsequent text uses new attributes; existing cells unchanged |
| Program completion or error | Reveal text page, show diagnostics if any, then prompt |
| SAVE/LOAD and binary transfers | Keep file byte streams separate from console output |
| Loaded source file in window mode | Load silently, run, then leave window at BASIC prompt |
| QUIT or window close | End session and release audio/video resources |
| Terminal EOF | Ends serial session; unrelated stdin EOF does not close a screen session |

The INPUT behavior above is the proposed completion of the agreed picture-mode
rule: the user must be able to see an interactive question. Record and test the
saved mode around successful INPUT, including consecutive INPUT statements.

Use 40 columns and 24 rows. Adopt immediate wrapping at the right edge; a
subsequent LF advances another line, including after an exact-width output.
LF clears the remainder of the current row using current colors and advances.
CR is ignored; keyboard Enter produces LF. Initially TAB emits one space,
matching the prototype. Scrolling preserves moved attributes and clears the
new bottom row with current colors. Unsupported glyph bytes retain existing
font behavior. Make these choices explicit in the manual.

Editing initially supports printable ASCII, Enter, and Backspace across wrapped
input lines. Backspace cannot erase the prompt or earlier output. Do not promise
cursor navigation, command history, or full-screen editing in this milestone.
Handle input-length limits without corrupting an accepted line. Paste is bounded
and uses the same character and line rules as typing. Ctrl-C cancels current
input or interrupts a running program, preserves the program, and returns to
the prompt; it must not require closing the application.

## 1. Finish feasibility before production integration

Extend the isolated candidate under `crates/rx82/notes/screen-console-budget/`.
Audit runtime RAM, register preservation, stack use, tokenizing input, file
input, and preload paths before assigning additional state. Prototype the
complete minimum interaction: echo, backspace, cursor, INPUT mode restoration,
and break recovery. Include all native hooks in the assembled measurement.

The current command reader tokenizes incrementally. Decide where editable input
is buffered before committing tokens, so deleting characters cannot leave stale
tokens or corrupt a long string. Preserve existing long-line and string limits.
Do not implement backspace solely by moving the display cursor. Reuse existing
input storage where possible; document any allocation within existing RAM.

Distinguish keyboard input from preload and file input explicitly. Echo applies
to interactive screen input only. LOAD, startup source, serial terminal echo,
and SAVE serialization must not acquire extra characters. A route flag alone
does not distinguish these sources. Define initialization, reset, and lifetime
for any additional flags without silently changing FF00–FF02 semantics.

Investigate two implementation risks while the candidate is still isolated:

- **Presentation during hidden-page writes.** FF33 accesses the selected page,
  so the prototype temporarily selects text mode. Live rendering can sample
  that transient state. Measure this at normal speed and turbo, including a
  scroll, and choose a presentation strategy that leaves guest register and
  timing behavior intact. If presentation is deferred until an emitter finishes,
  specify the bounded deferral and how the frontend identifies completion;
  do not suppress intentional SCREEN changes. Validate with captured frames.
  Do not claim final register restoration proves flicker-free output.
- **Cursor ownership.** Prefer a guest-managed cursor with saved underlying cell
  and attributes, visible while waiting for interactive input. Measure hide/show
  and editing hooks. A renderer overlay is an alternative only with a documented
  way to obtain cursor state; it must not rewrite VRAM or infer position from
  terminal output. Blinking is optional for this milestone; a visible cursor is
  required. Device/audio time must continue while awaiting input.

If the complete minimum exceeds EF00, make targeted native code savings and
rerun semantic checks. Report the exact shortfall and savings, keeping HELP
useful. Do not shrink string RAM, relocate regions, add banks, or expand ROM.
Do not fall back to implementing native console behavior in the host without
explicitly revisiting the design.

**Exit:** assembled minimum fits; RAM/state ownership and editing design are
recorded; presentation and cursor choices have working demonstrations. Update
the feasibility report with remaining headroom and any unresolved limitation.

## 2. Integrate native console behavior

Move the accepted routines and hooks into `sys/basic_rom.asm`, initialize all
console state before the banner, and regenerate `sys/basic_rom.bin`. Add an
explicit host configuration path for screen/serial mode; preserve serial as the
default for existing low-level native machine APIs and headless callers.

Route shared character output after the existing file-output dispatch. Preserve
registers expected by PRINT, listings, diagnostics, and file helpers. Keep
hardware VRAM pointer and mode preservation around screen emission. Reset the
text cursor only at boot and text CLS. Centralize return-to-prompt mode handling
so normal END, STOP, errors, and break agree.

Implement interactive entry/echo, protected backspace, cursor visibility, and
INPUT restoration using the design proved in step 1. Hide the cursor before
program output and page changes. Make cancellation clean up parser, execution,
file, and temporary-string state through existing recovery paths where possible.
Break must work while executing CPU instructions, not only while polling input;
define a controlled guest recovery mechanism and measure its native cost.

**Exit:** full native screen sessions work through deterministic input queues;
serial and file behavior remain covered; source/binary equality and size pass.

## 3. Connect the window keyboard and console selection

Extend `src/live.rs` so the SDL event loop forwards text-entry, Enter, Backspace,
and break events to the emulation worker through a bounded channel. Respect the
SDL thread model and distinguish text events from control key events to avoid
duplicate characters. Handle repeat, focus, and unsupported characters
consistently. Do not send keyboard input to both the screen and serial paths.

Screen sessions own window input and do not start the blocking stdin reader.
Serial sessions retain terminal transport. Closing the window must wake or stop
a worker waiting for input, including when channels are full. Preserve existing
4 MHz pacing, device advancement while waiting, bounded audio, and turbo rules.
Integrate the presentation strategy established in step 1.

Proposed CLI contract:

- `rx82 basic --live`: native screen session by default.
- `rx82 basic --live --console serial`: native window with terminal console.
- `rx82 basic --native`: existing headless native serial session.
- `rx82 basic`: existing headless reference serial session.
- `--reference` explicitly selects the reference runner for a live session;
  reject combinations with `--native`.
- `--console screen` requires `--live` in the user-facing CLI; tests can construct
  a headless screen device directly.

Resolve runner selection in one place in `src/main.rs`. Preserve documented
monitor restrictions and explain any invalid combination through CLI errors.
Window source-file sessions return to the prompt after execution; existing batch
headless termination stays intact. Document the live-default change clearly.

**Exit:** a user can launch, enter and edit a program, RUN, answer INPUT, break,
LIST, and quit without an attached terminal.

## 4. Match reference-runner behavior

Add a console abstraction to `src/basic.rs` and its live adapters covering output,
cursor state, interactive input, and mode transitions. Keep file serialization
independent. Match screen dimensions, controls, wrapping, colors, scrolling,
INPUT restoration, and prompt behavior from the native contract.

Keep the native runner as the acceptance target. Reference support must not mask
an unimplemented guest path. Compare observable text/picture pages and console
behavior; CPU instruction counts need not match. Keep existing BufRead/Write
serial use available to tests and callers.

**Exit:** shared native/reference scenarios agree and live reference selection is
explicit and documented.

## 5. Validate and publish the new contract

Automated checks should cover observable behavior and integration boundaries:

- ROM capacity, branch assembly, source/binary equality, and state initialization.
- Banner, PRINT formatting, INPUT, LIST, HELP, errors, END/STOP/break, and repeated
  runs through both console routes.
- Exact 40-column boundary behavior, multiple scrolling passes, moved attributes,
  CLS, COLOR, TAB/CR/LF, and retained cursor across SCREEN changes.
- Hidden text output preserving picture bytes and VRAM pointer; captured live
  frames during emission/scroll; legitimate SCREEN transitions still visible.
- Enter, backspace across a wrap, prompt protection, empty lines, maximum input,
  long string/token input, paste limits, and cancellation at prompt/INPUT/RUN.
- Silent source preload, exact SAVE bytes, LOAD and BLOAD/BSAVE regressions, and
  serial sessions with existing tests and examples.
- Cursor restoration without damaged cells; input waiting keeps sound and device
  timing alive; bounded channels and clean shutdown on EOF/QUIT/window close.
- Native/reference agreement and CLI default/option validation.

Run focused checks as components land, then the repository's normal required
checks for the integrated change. Use SDL dummy drivers for automated event and
frame checks where supported. Perform a real window/keyboard/audio session for
focus, responsiveness, cursor visibility, break latency, and presentation at
normal speed and turbo. If physical validation is unavailable, record it as
outstanding rather than describing dummy-driver tests as equivalent.

Update `crates/rx82/README.md`, examples, native HELP, `video-sound-spec.md`, and
`video-sound-plan.md` to replace the serial-only rule with the implemented console
contract. Mark older deferred cursor/output statements as superseded, retaining
historical measurements. Update the prototype report to point to the final code
and measured ROM size. Include a short runnable example combining picture output,
retained PRINT output, sound, and return to the prompt.

**Done:** the complete native interactive screen console fits the unchanged map,
works without a terminal, retains explicit serial operation, passes the stated
checks, and has matching documentation. Any unavailable physical validation is
called out separately.
