# BASIC BLOAD / BSAVE implementation plan

Status: proposed; this document does not implement the commands.

## Agreed behavior

- `BSAVE "file.bin",address,length` writes exactly `length` raw bytes.
- `BLOAD "file.bin",address` loads the entire raw file at `address`.
- Files have no header, embedded address, encoding, or BASIC serialization.
- Both commands work at the prompt and in programs, in native ROM BASIC and
  the Rust reference interpreter.
- Failed BLOAD validation writes no file payload into destination memory.

## Proposed defaults

Memory access policy was not explicitly selected. This plan proposes allowing
any range contained entirely within either RX-82 RAM region: `0000–BFFF` or
the BASIC string RAM at `EF00–FEFF`. Reject ROM, devices, gaps, and transfers
crossing a region boundary. Apply the same bounds in the reference interpreter
even though its backing byte memory spans 64 KiB. These restrictions apply to
both commands. Confirm this policy before implementation.

Interpreter-owned RAM remains accessible, consistent with existing POKE
behavior. Successful native loads over variables, strings, program records,
scratch, or the CPU stack can corrupt BASIC or prevent the command returning.
This feature does not promise safe restoration of an interpreter snapshot.
Use `0100–01FF` for small examples and normal execution tests. Reference byte
memory does not acquire native variable, stack, or string-pool semantics.

Other defaults:

- Filenames are nonempty quoted literals, preserving case and spaces; relative
  paths use the host working directory. Reuse the native file port's 240-byte
  filename limit in both implementations. String expressions are outside scope.
- Addresses follow PEEK/POKE: bare unsigned literals through 65535, otherwise
  checked signed 16-bit expressions reinterpreted as address bits.
- Length accepts bare unsigned literals through 65535 or nonnegative signed
  expressions. Negative lengths are errors, not unsigned aliases. An expression
  cannot bypass normal signed arithmetic overflow checks.
- Validate bounds using a widened sum or a remaining-capacity comparison;
  never permit 16-bit wraparound. Each nonempty transfer must fit one region.
- Zero length saves an empty file; an empty file loads zero bytes. Still require
  an address in an allowed RAM region and valid syntax and filename.
- BSAVE replaces an existing file, matching SAVE. Parse and validate all
  arguments before opening output. Host write failures retain the existing
  file device's semantics; atomic filesystem replacement is outside scope.
- Commands do not intentionally reset variables, arrays, strings, program
  records, DATA position, or control-flow frames. Successful payload writes
  may themselves modify native state when the destination overlaps it.

## Current integration points

- `crates/rx82/sys/basic_rom.asm`: statement dispatch, `FILENAME`, `SAVE`,
  `LOAD`, `MEM_ADDRESS`, file error recovery, tokenizer, and `TOKEN_TABLE`.
- `crates/rx82/src/basic_tokens.rs`: host codec derives keywords from the ROM
  table; encoding tracks unsigned address contexts.
- `crates/rx82/src/basic.rs`: reference `statement_body`, `Parser::address`,
  byte memory, and prompt-only text SAVE/LOAD handling.
- `crates/rx82/src/files.rs`: raw file port buffers input on open and supports
  rewind; output is buffered until close. The existing file size cap is 1 MiB.
- `crates/rx82/src/native.rs` and integration tests: native sessions and ROM
  source/image checks.

Keep host file handling generic. Native parsing, range validation, and copying
execute on the R8 CPU using the existing byte-stream device; no BASIC-aware
host command or direct host memory injection is needed.

## Implementation sequence

### 1. Tokens and argument parsing

Allocate two unused keyword codes without renumbering existing tokens or
colliding with payload/operator tokens. Add dispatch for both commands to the
ordinary statement path, including statements reached through IF THEN.

Split native filename parsing into a reusable quoted-literal parser and a
wrapper retaining the direct-mode and end-of-statement requirements of text
SAVE/LOAD. Binary commands consume commas and numeric arguments themselves.
Reject empty filenames explicitly, missing arguments, extra arguments,
unterminated strings, and trailing tokens before opening a file.

Extend address parsing to accept a bare unsigned literal at end of statement:
native `MEM_ADDRESS` currently recognizes comma and closing parenthesis as
literal terminators. Preserve existing expression and PEEK/POKE behavior.
Implement a separate length parser or parameterized unsigned-literal helper
so negative lengths cannot inherit address bit-pattern semantics.

Update both tokenizers' numeric context handling. BLOAD/BSAVE numeric arguments
follow a quoted filename and commas, so simply marking the keyword as an
address context is insufficient. Track argument positions and parenthesis
depth, retaining normal rules for nested expressions and unrelated statements.
Check the host decoder's handling of unsigned literal payloads as well as
encoding. Ensure LIST and text SAVE/LOAD round-trip the new statements.

### 2. Reference interpreter

Implement both commands in `statement_body`, making them available through
the existing prompt and program execution paths. Extract parsed arguments
before taking mutable access to reference memory.

For BSAVE, validate the range, collect exactly the requested bytes from the
reference memory map (missing entries read as zero), then write the file.
For BLOAD, read the file into a bounded buffer, validate its actual length
against the destination capacity, and only then insert bytes into memory.
Use bounded reading to detect oversize input without allocating arbitrarily
large host buffers; do not rely solely on filesystem metadata.

Do not call text program loading, clear variables, rebuild program records,
or route bytes through device registers. Preserve the existing distinction
between reference byte memory and interpreter data structures.

### 3. Native BSAVE

Parse all arguments, validate the entire range, then open the file for writing.
Walk memory with a pointer and remaining-byte count; transfer bytes through
FF13 and check status. Close to commit only after all requested bytes have
been accepted. Zero length still opens and closes output.

Preserve the statement cursor and calling convention, including program,
GOSUB, and FOR execution. Allocate and document transfer scratch after auditing
the actual zero-page layout; do not assume adjacent bytes are free. Reads of
interpreter scratch or stack reflect the running machine, not a frozen snapshot.

### 4. Native BLOAD and validation guarantee

Use two passes over the file port's buffered input:

1. Parse the complete command and validate the starting address.
2. Open input; check status before interpreting EOF.
3. Count bytes without destination writes. Reject immediately when the count
   exceeds the selected RAM region's remaining capacity, avoiding counter wrap.
4. On successful validation, rewind the same buffered input and check status.
5. Copy the validated number of bytes into RAM, then close input.

Rewind must use the original buffered bytes, not reopen the pathname. Changes
to the host file after open therefore cannot alter the validated payload.
Review FilePort's existing status/EOF behavior and preserve its protocol.

Syntax errors, bad filenames, read/open failures, and invalid ranges must abort
before copying. Common error recovery must abort file state, retain useful
line context, and return to the prompt with a balanced stack. No rollback is
promised after copying begins if a load overwrites active interpreter state.

The unchanged-memory guarantee means no payload destination writes on failed
validation, with ordinary BASIC state preserved apart from existing expression
evaluation effects. It cannot mean that every physical RAM byte stays frozen:
tokenization, stack operations, scratch, and diagnostics necessarily use RAM.
Expressions invoking RND, for example, retain their ordinary device effects.

### 5. Diagnostics, documentation, and ROM

Reuse existing filename, comma, numeric overflow, and file diagnostics where
appropriate. Add clear length/range errors rather than reporting a malformed
transfer as a file failure. Document corresponding reference errors without
requiring identical capitalization or formatting across interpreters.

Update both HELP outputs and `crates/rx82/README.md` with syntax, raw format,
length rules, memory bounds, program use, failure behavior, and reference/native
memory differences. Add a small example saving bytes from `0100–01FF`, changing
them, loading them back, and observing them with PEEK.

Reassemble `crates/rx82/sys/basic_rom.asm` using the existing assembler command.
Verify the emitted image path and include the generated BASIC image. Measure
remaining ROM headroom and require the image to end before string RAM at EF00.

## Validation during implementation

Add focused coverage using existing session and temporary-file helpers:

- Exact raw output and round trip for bytes 0, 10, 13, 127, 128, and 255;
  embedded zeroes must never terminate a transfer.
- Prompt execution, numbered programs, IF THEN, loops, and GOSUB continuation.
- Variables and expressions for address/length; signed address aliases; bare
  unsigned literals above 32767 in prompt input and tokenized program lines.
- Text LIST/SAVE/LOAD round trips without keyword or numeric-token regressions.
- Zero length, empty input, one-byte transfers, exact region endpoints,
  one-byte overruns, crossing gaps, ROM/device addresses, and address wrap.
- Large unsigned lengths and rejection of negative or overflowing expressions.
- Quoted paths with spaces/case; missing/empty/oversize filenames; missing
  commas or operands; extra operands and trailing tokens.
- Missing/unreadable input and oversize payload: prefill destination with a
  sentinel, attempt BLOAD, and verify no payload bytes changed. Verify existing
  variables, program, arrays, strings, and DATA cursor survive validation errors.
- Invalid BSAVE syntax/range does not create or truncate a file; valid saves
  replace existing content with exactly the requested bytes.
- File errors recover to the prompt; a later valid transfer succeeds.
- Cross-interpreter file interchange in shared safe RAM, checking identical
  bytes rather than equality of interpreter-owned state.

Use low-level range/helper coverage for destructive regions such as the native
stack, and small sessions for ranges that can safely be accessed. Do not require
a native session to continue normally after loading arbitrary stack contents.
Inspect byte memory directly for failed-validation sentinel checks where needed.

After targeted checks and ROM regeneration, run:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features
cargo test --all-features
```

## Acceptance criteria

- Both signatures execute in both interpreters at the prompt and in programs.
- Binary files contain exactly the requested bytes and interchange correctly.
- Parsing and range validation finish before output creation or payload writes.
- Failed BLOAD validation preserves destination payload bytes and normal BASIC
  state, subject to documented evaluation and scratch-memory behavior.
- The agreed memory policy, zero-length cases, and numeric boundaries are
  enforced consistently, with no accidental device access or address wrapping.
- Existing text SAVE/LOAD behavior and PEEK/POKE semantics remain compatible.
- Documentation, generated ROM, focused coverage, and repository checks agree.
