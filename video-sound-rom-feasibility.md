# Video and sound: native ROM feasibility

Assessment date: 2026-10-09. Contract: `video-sound-spec.md`.

## Screen-console follow-up: measured output core

On 2026-10-09 an isolated native output-console candidate assembled to **11,572
bytes**, exclusive end **EE34**, leaving **204 bytes before EF00**. Net growth
is **240 bytes**: 235 bytes of routines, 9 bytes of integration calls, and a
4-byte HELP text saving. It keeps the existing memory map and uses four unused
system RAM bytes for cursor position and serial/screen selection.

Eleven native sessions passed against the actual video device, including exact
scroll rows, picture retention, mode/pointer restoration, and SAVE serialization.
This measures output, wrapping, scrolling, colors, serial selection and prompt
return to text mode. Keyboard integration, input echo/editing and a visible
cursor remain unmeasured; temporary video mode switching also needs live-frame
assessment. It is not a claim that the complete interactive frontend fits or is
ready to ship. Reproduction and limitations are in
[`screen-console-budget/README.md`](crates/rx82/notes/screen-console-budget/README.md).

## Reduced milestone: measured fit

**The restrained native interface fits the current ROM window.** A complete
assembled capacity candidate adds **355 bytes**, yielding **11332 bytes** total
and **444 bytes free** before EF00. Its inclusive last address is ED43; ED44 is
the exclusive end. No further ROM compaction or memory-map change is needed
for this candidate.

The candidate implements SCREEN, CLS, COLOR, and PLOT with the existing native
expression parser, bounds checks, complete-statement validation, device reads
and writes, clear loops, and error recovery. It includes four keyword records,
four dispatch entries, native LIST/SAVE detokenizer spacing, two HELP lines,
and the ILLEGAL QUANTITY diagnostic. It needs no persistent RAM scratch.

| Measured addition | Bytes |
| :--- | ---: |
| Four dispatch entries | 44 |
| Four keyword records | 26 |
| HELP additions | 92 |
| Native detokenizer spacing | 8 |
| SCREEN routine | 15 |
| COLOR routine | 26 |
| PLOT routine | 45 |
| Shared argument validation and error dispatch | 14 |
| ILLEGAL QUANTITY text | 19 |
| CLS, both page-fill paths | 66 |
| **Total actual ROM growth** | **355** |

The whole candidate was assembled after all insertions; every short branch
still fits, so this layout requires no extra branch expansion. Shared existing
EXPR, EOL, BINARY_COMMA, MATCH, and REPORT_ERROR routines are reused and not
counted twice. Arguments are preserved on the existing CPU stack until syntax
and bounds are checked. CLS derives mode/colors from device readback, and PLOT
checks picture mode before writes. PRINT remains serial.

Use **355 bytes as the demonstrated requirement** for this implementation.
For planning, reserve another **128 bytes** for integration changes, giving a
**483-byte working allocation** and **316 bytes of unallocated headroom**.
The 128-byte reserve is an allowance, not emitted code or a newly measured
requirement. Recheck the final image after any changes to helpers or layout.

### Reproducible candidate and verification

The isolated prototype lives in `crates/rx82/notes/video-sound-budget/` and does
not alter the shipped ROM or expose commands before devices are implemented.
Run from the repository root:

```sh
python3 crates/rx82/notes/video-sound-budget/verify.py
```

The script assembles baseline and candidate using the repository assembler,
checks baseline source/image equality, emits symbol maps, and runs 19 native
sessions. All 19 passed: exact command port writes; 1920/3840-byte clears;
range and syntax rejection without video writes; error recovery; serial PRINT;
new tokens through native LIST; and expressions, IF, FOR, GOSUB, and RETURN.
The checks caught and fixed the missing detokenizer spaces, whose eight bytes
are included above. The final candidate is 11332 bytes against a 11776-byte
window, compared with the current shipped baseline of 10977 bytes.

A port recorder supplies mode/color readback and captures writes. These checks
validate native routines and capacity, not a complete video controller, audio
backend, or host interpreter parity. Those remain implementation work. Device
rendering, synthesis, polling clock, and host integration consume no guest ROM;
this milestone adds no native sound wrapper, cursor, scroll, LINE, or IRQ code.

## Historical broader-scope assessment

The measurements of the existing ROM and completed 372-byte compaction below
remain valid. The former 2048-byte allocation, 1249-byte shortfall, and further
restructuring recommendations concern the superseded broader feature set.
They do not apply to the restrained milestone measured above.

## Original decision (broader scope)

**The current ROM budget does not support starting the complete native feature.**
Safe local compaction has recovered 372 bytes, leaving 799 bytes. Reserve 2048
bytes for the new native work, including integration allowance: the remaining
capacity shortfall is 1249 bytes. A further ROM restructuring pass is needed
before proceeding with the feature under the frozen memory map.

This is a capacity gate against an explicit engineering budget, not a proof
that no possible implementation can fit. The new routines are not implemented
or assembled yet; their budgets below are estimates. Do not describe the
complete feature as demonstrated to fit, or the fixed map as impossible.

## Measured baseline and applied compaction

The actual assembler, rather than source-line counts, produced these sizes:

| Image | Bytes | Inclusive last address | Free before EF00 |
| :--- | ---: | :--- | ---: |
| Original checked-in BASIC | 11349 | ED54 | 427 |
| Short conditional branches | 10995 | EBF2 | 781 |
| Short branches plus direct returns | 10977 | EBE0 | 799 |

The window C100–EEFF has exactly 11776 bytes. Source and generated binary have
both been updated for the final row. No main-RAM region or existing device
contract changed. The device feature itself has not been implemented.

Changes applied:

- Replace 118 inverted-branch/absolute-jump sequences with a direct short
  conditional branch: 3 bytes saved each, 354 total. An assembler-label-based
  probe found 116 in its first pass and 2 more after addresses shortened.
  Each displacement was checked by reassembly; skip labels were retained.
- Replace nine `jmp DONE` instructions with `ret`: 2 bytes saved each, 18 total.
  DONE is exactly one `ret`; both paths have identical register/flag effects.

For example, `bne skip; jmp target; skip:` becomes `beq target; skip:` only
when the relative displacement fits. This changes cycle counts while preserving
BASIC behavior. Future inserted code must be reassembled: short branches near
their limits may need expansion, which is covered by the integration allowance.

## Measurements informing the feature budget

These are assembled spans in the compacted image, not estimates:

| Existing span | Bytes | Relevance |
| :--- | ---: | :--- |
| STATEMENT_BODY to DONE | 298 | Current statement dispatch |
| PRINT to EXPR | 64 | PRINT parsing alone, excluding emitters |
| PRINT_NUM to PEEK | 93 | Existing numeric formatting to reuse |
| PUTCHAR to NEWLINE | 26 | Existing file/console output routing |
| MEM_ADDRESS to POKE_BYTE | 80 | Existing address parsing helper |
| POKE_BYTE to RANDOMIZE | 93 | Existing two-argument checked byte write |
| BSAVE to BLOAD | 130 | One streaming command with parsing/helpers |
| HELP_TEXT to INVALID_LINE_NUMBER | 599 | Help text, potential compression target |
| TOKEN_TABLE to TOKEN_TABLE_END | 244 | Existing keyword records |

The font, RGB palette, video RAM, audio synthesis, window backend, and CPU trap
plumbing are host/device responsibilities and consume **no BASIC ROM bytes**.
Conversely, text scrolling, PRINT routing, command validation, Bresenham, and
the BASIC interrupt handler must occupy real guest ROM. Offloading those to
BASIC-aware device commands would violate the frozen contract.

## Native feature capacity allocation

These are implementation ceilings to design toward, **not measured routine
sizes**. Existing expression evaluation, numeric formatting, string output,
and syntax-error infrastructure are reused. The allocation includes new
helpers but does not count those reused routines again.

| Addition | Budget bytes | Contents |
| :--- | ---: | :--- |
| Keyword records | 40 | Exact: 28 name bytes plus six token/terminator pairs |
| Dispatch entries | 66 | Six entries at the existing 11-byte distant-target pattern |
| Shared parsing/range checks | 240 | Commas, signed bounds, mode checks, full validation before writes |
| SCREEN, COLOR, CLS | 192 | Mode/color writes, two page-fill paths, cursor reset |
| Text emission and PRINT routing | 288 | Attributes, pointer setup, cursor, TAB/CR/LF, wrap, output destination |
| Text scrolling | 192 | Port reads/copies, last-row fill, register preservation |
| PLOT and LINE | 320 | Port sequence, inclusive signed Bresenham, saved arguments |
| SOUND | 192 | Period byte writes, register read/modify/write, packed volumes |
| Interrupt startup and handler | 96 | Vector installation, enable, register save, frame counter, RTI |
| Diagnostics and HELP additions | 128 | ILLEGAL QUANTITY, syntax/help text |
| Integration allowance | 294 | Branch expansion, reset/error paths, calling conventions, scratch access |
| **Total** | **2048** | |

With this allocation the base interpreter must be at most 9728 bytes (2600 hex)
before feature insertion. It is currently 10977 bytes, so another **1249 bytes**
must be recovered. Filling only the 799 free bytes would consume less than half
of this allocation and leave no credible integration margin.

Bresenham and text output are the largest uncertainty. A smaller budget is
acceptable only after assembling representative complete routines, including
argument validation, preservation, scrolling, and error exits. Empty stubs,
DATA padding, or host implementations do not establish native feasibility.

## Further options, without silently changing the contract

1. **Continue with structural ROM compaction (recommended).** Audit duplicated
   string/parser paths, dispatch, and shared error/output tails; consider a
   compact encoding for ROM help text with a guest decoder. Keep existing
   language behavior and output, and measure each rewrite. Target at least
   1249 additional bytes under the current allocation. Savings are not yet
   demonstrated; this is a separate, substantial interpreter change.
2. **Prototype the largest routines and revise the budget using assembly.**
   Implement complete isolated text/scroll, LINE, and SOUND routines against
   real calling conventions, then sum their emitted sizes. This may lower or
   raise the shortfall; it must not be reported as a saving in advance.
3. **Revise the architecture only if the first two approaches cannot close
   the gap.** A banked ROM adds guest-visible bank state and interrupt concerns;
   moving string RAM contradicts the fixed map; moving BASIC work into devices
   contradicts the native execution contract. None is authorized by this
   assessment and none has been introduced.

For perspective, even deleting the entire 599-byte HELP text would leave only
1398 bytes free, still 650 bytes below the allocation—and would remove existing
behavior. Compression recovers less after counting its decoder. Dispatch is
only 298 bytes in total; it cannot alone close the gap. These measurements
rule out treating help trimming or dispatch cleanup as a sufficient solution.

## Validation and reproduction

The compaction was assembled using this repository's `r8asm::Assembler`, which
also exposes label addresses for the span measurements. The regenerated image
is `crates/rx82/sys/basic_rom.bin`. Standard reproduction:

```sh
cargo run -p r8asm -- crates/rx82/sys/basic_rom.asm
wc -c crates/rx82/sys/basic_rom.bin
cargo test -p rx82 --all-features
```

Expected binary size is 10977 bytes; C100 + 10977 = EBE1 is the exclusive end.
The existing `rom_matches_source_and_fits` test checks exact source/image equality
and the EF00 bound. `cargo test -p rx82 --all-features` passed all 187 tests
(143 unit tests, 43 integration tests, and one doctest), including the existing
arithmetic cycle regressions. `git diff --check` also passed. No new tests were
needed for these mechanically equivalent control-flow substitutions.
