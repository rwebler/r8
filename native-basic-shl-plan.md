# Native BASIC SHL implementation plan

Status: proposed; no interpreter changes implemented by this document.

## Objective

Use the existing R8 `SHL` instruction to reduce instructions and ROM space in
`crates/rx82/sys/basic_rom.asm`, preserving BASIC behavior, checked signed
arithmetic, memory layout, register preservation, and error recovery.

The ROM currently contains 12 self-add instructions. Multiplication already
uses binary shift-and-add, and division already uses divisor alignment and
binary subtraction. This work optimizes their doubling operations rather than
replacing those algorithms.

## Instruction semantics and constraints

For an 8-bit register or 16-bit register pair, replace:

```asm
    clc
    add ef, ef
```

with:

```asm
    shl ef, 0x01
```

`SHL` ignores incoming carry, updates zero and negative from the result, and
sets carry to the bit shifted out. For a one-bit shift, the result and flags
match the original pair. Verify against `crates/r8cpu/src/logic.rs` and
`crates/rx82/src/cpu.rs` during implementation. No CPU or assembler extension
is needed.

For multi-bit shifts, carry reflects only the last bit shifted out; it is not
an aggregate overflow indicator. Use a multi-bit shift only where bounds prove
that intermediate bits cannot be lost. Keep `CLC` before subsequent ordinary
additions: `ADD` consumes carry, including carry produced by `SHL`.

## Complete candidate inventory

Locations below are assembly labels, so they remain useful as line numbers move.

| Location | Existing self-adds | Planned replacement | Constraint |
| --- | ---: | --- | --- |
| `MUL_SHIFT` | 1 | `shl ef, 0x01` | Preserve `bcc MUL_LOOP` and overflow path. Keep the zero-multiplier guard before shifting. |
| `DIV_ALIGN` | 2 | `shl ef, 0x01` and `shl gh, 0x01` | Keep alignment comparisons and quotient-mask behavior. Magnitudes are at most `0x8000`; the doubled divisor is smaller than the dividend before shifting. |
| `VARIABLE` | 1 | `shl a, 0x01` | The validated A–Z index is 0–25, so its doubled byte offset fits. |
| `NUMBER_ACCUMULATE` | 3 | One two-bit shift and one one-bit shift | Preserve decimal bounds and both existing carry checks; see below. |
| `ARRAY_DESCRIPTOR` | 1 | `shl ab, 0x01` | Preserve mapping from scalar addresses to four-byte array descriptors. |
| `LOCATION_IN_RANGE` | 1 | `shl ab, 0x01` | Retain subscript validation and carry clear before adding the array base. |
| `DIM_SIZE_OK` | 1 | `shl ab, 0x01` | Keep increment-before-shift for `(upper_bound + 1) * 2`, the size bound, and allocation checks. |
| `POOL_REWRITE_ARRAY` | 1 | `shl ab, 0x01` | Preserve the exclusive end address for string-array handle rewriting during pool compaction. |
| `RANDOM_REMAINDER_ALIGN` | 1 | `shl ef, 0x01` | Retain the existing dividend/divisor range assumptions, alignment comparison, and remainder loop. |

The 12 self-adds become 11 shifts because two adjacent decimal-parser doublings
collapse into one instruction.

## Decimal accumulation

Keep the existing `10x = (4x + x) * 2` structure:

```asm
NUMBER_ACCUMULATE:
    ld cd, ef
    shl ef, 0x02
    clc
    add ef, cd
    shl ef, 0x01
    bcc LONG_652
    jmp OVERFLOW
LONG_652:
    clc
    add ef, ab
    bcc LONG_655
    jmp OVERFLOW
```

The existing precheck limits the accumulator to 6553 (`0x1999`) and permits
only digits 0–5 at that boundary. Consequently `4x <= 26212`, `5x <= 32765`,
and `10x + digit <= 65535`. This proves the two-bit shift is safe. Preserve
the distinction between unsigned number parsing and signed expression limits.
Tokenized numeric literals bypass this text accumulation path; measure parser
benefits on a path that actually reaches `NUMBER_TEXT`.

## Implementation sequence

1. Record the current ROM size and isolated multiplication/division cycle
   counts. Save a reproducible baseline of the current binary algorithms for
   comparison; existing legacy fixtures benchmark repeated addition/subtraction
   and do not isolate the incremental SHL improvement.
2. Apply the one-bit replacements in multiplication, division, and random
   remainder. Preserve all guards, branches, sign helpers, and register saves.
3. Apply variable, descriptor, array allocation/access, and pool traversal
   replacements. Review each subsequent flag consumer and base-address addition.
4. Apply the decimal accumulation rewrite, retaining the bound checks and
   documenting why the two-bit shift cannot discard significant bits.
5. Rebuild the checked-in ROM image:

   ```sh
   cargo run -p rx82 -- asm crates/rx82/sys/basic_rom.asm
   ```

6. Run correctness and performance validation below. Update the native BASIC
   arithmetic description in `crates/rx82/README.md` with SHL usage and measured
   results, clearly distinguishing current-algorithm and legacy comparisons.

## Validation

Reuse existing meaningful coverage; add cases only where coverage is missing.

- Multiplication: compare against checked `i16` results, including zero,
  positive/negative products, `-32768 * 1`, `-32768 * -1`, and overflowing
  products. Keep the last-bit guard: an unnecessary final doubling can reject
  a valid result.
- Division: preserve truncation toward zero, division-by-zero errors,
  `-32768 / -1` overflow, and boundary divisors. Verify saved registers and
  balanced stack through the existing isolated harness.
- Decimal parsing: directly exercise `NUMBER_TEXT` with 0, 32767, 32768,
  65535, and rejected 65536; separately verify signed expression handling,
  including `-32768`. Ensure tokenization does not bypass the target routine
  in these tests.
- Variables and arrays: verify A/Z scalar offsets, descriptor mapping,
  zero-based and upper-bound element access, nested indices, allocation limits,
  and recovery from invalid dimensions/subscripts.
- String pool: exercise compaction with live string-array entries, checking
  that every handle is rewritten and traversal stops at the correct end.
- Random remainder: exercise boundary bounds, rejection sampling, and existing
  deterministic seeded sequences; output must remain identical.
- ROM integration: verify source/image agreement and expansion-window fit.
  Assembly must succeed with relocated labels and branch offsets.

Run the existing isolated cycle reports before and after the edits:

```sh
cargo test -p rx82 native_multiplication_cycle_regression -- --nocapture
cargo test -p rx82 native_division_cycle_regression -- --nocapture
```

Use the same operands and harness for the saved pre-SHL comparison. Include
cases that actually execute doubling; zero or immediate-exit cases may have
unchanged cycle counts. Report R8 cycles and ROM bytes, not unmeasured host
wall-clock speedups. Existing isolated counts include harness/sign helpers and
exclude BASIC parsing and printing. If measuring decimal parsing, identify its
separate workload explicitly.

Complete repository checks after rebuilding the image:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features
cargo test --all-features
```

## Acceptance criteria

- All 12 identified self-adds are replaced by the planned 11 shifts, with
  required carry clears for ordinary additions retained.
- BASIC results, errors, memory layout, seeded random output, and register/stack
  contracts remain unchanged.
- The rebuilt ROM agrees with source and fits its reserved window.
- Correctness checks pass, and measured comparisons demonstrate savings on
  paths executing the optimized operations without regressions elsewhere.
- The final change includes assembly, generated ROM, any necessary coverage,
  and updated documentation with reproducible measurements.
