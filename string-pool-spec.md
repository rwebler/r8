# RX-82 BASIC string pool, string arrays, and string stack

Status: implemented with the approved layout revisions. Existing scalar and
array behavior and the stock firmware are preserved. Both runners use native
temporary-space accounting and report `? STRING SPACE` and `? STRING TOO LONG`.

## Memory

| Range | Use |
| :--- | :--- |
| `0300`–`0333` | 26 integer scalar words. |
| `0340`–`0373` | 26 string scalar offsets. Zero means empty. |
| `0800`–`0867` | 26 integer-array descriptors: absolute base and inclusive bound. |
| `0870`–`08D7` | 26 string-array descriptors, with the same layout. |
| `0900`–`0FFF` | Scratch only; no persistent strings. |
| `1000`–`8FFF` | Program records, unchanged. |
| `9000`–`9FFF` | Shared integer/string array elements, 2048 words total. |
| `A000`–`BFFF` | CPU stack, growing downward from `BFFF`. |
| `C000`–`C0FF` | Stock firmware, unchanged. |
| `C100`–`EEFF` | Space for the optional BASIC ROM, entry at `C100`. |
| `EF00`–`FEFF` | Optional 4096-byte string RAM, installed with BASIC. |
| `00B2` | Pool bump, initially `EF00`. |
| `00B6` | String-stack top, initially `FF00`. |
| `00B8` | Statement mark, a saved stack top. |
| `00C0` | Compaction-needed flag. |
| `00C1`–`00C9` | Allocator/compactor work state. |
| `00BC`–`00BD` | Relocated DATA next-item pointer. |
| `00BE` | DATA quoted flag. |
| `00BF` | IF operand-type flag. |
| `00D0` | Console line-length mode. |

The stock ROM source and binary are unchanged. The expansion interface permits
images starting at `C100`, checking that they overlap only zero firmware padding.
BASIC's code ends before the string RAM. The reset vector still enters `C000`.
Ordinary RX-82 machines do not install the additional RAM.

## Records

A live string is one length byte (1–255) followed by ASCII text, without NUL.
Empty strings have no stable record. Allowed bytes remain tab and `20`–`7E`.

Scalar and string-array words store `record_address - EF00 + 1`; zero means
empty. Thus the first record has offset 1. Compaction decodes with `EEFF + offset`.
Integer and string variables and arrays remain independent: `A`, `A$`, `A()`,
and `A$()` can all coexist.

A free record starts with zero, followed by its former payload length (1–255).
This preserves its total size while allowing live lengths 128–255. There is no
ambiguous flag bit in either string lengths or array addresses.

## Pool

`POOL_ALLOC` takes a nonzero length in B and returns the record address in CD.

1. Walk records from `EF00` to the bump pointer. Reuse the first fitting free
   record. An exact fit is accepted; a remainder of at least two bytes becomes
   another free record. Skip a block that would leave a one-byte remainder.
2. Otherwise bump if the new end is strictly below the string-stack top.
3. Otherwise report `? STRING SPACE`, preserving the old destination.

Replacing a value allocates and copies the new record **before** freeing the
old record. Assigning an empty string needs no new record. Freeing a record
sets the compaction flag. At the next statement boundary, if the temporary
stack is empty, compact before evaluating expressions. This makes compaction
possible without relocating any expression's live temporary pointers.

Compaction slides live records down, rewrites all scalar string offsets and
all dimensioned string-array elements, then updates the bump. Integer scalar
and array words are never scanned as string references. It does not erase
unused trailing bytes; only the range below the bump contains stable records.

`RUN`, `NEW`, and successful `LOAD` reset the pool, string stack, scalar offsets,
and array descriptors. Failed stores and failed `LOAD` preserve existing values.
`SAVE` writes source only.

## String stack

At statement start save the stack top as the statement mark. Expression pieces
push a length byte and their text downward; even an empty temporary costs one
byte. Concatenation validates the combined length, pops its two operands, and
pushes the join. The maximum result is 255 characters.

If a push would meet or cross the pool bump, report `? STRING SPACE`; there is
always at least one byte between the two regions. No compaction occurs while
temporaries exist. Assignment copies the top temporary into stable storage,
then pops it. `PRINT` and `LEN` pop their results without storing them; string
comparisons retain both operands until comparison. Statement completion and
error recovery restore the statement boundary. FOR iterations do not leak
string temporaries.

## Arrays

`DIM A$(N)` allocates N+1 zero offsets from the shared array area. A second DIM
of the same typed array is an error. The integer and string descriptor tables
identify element types without stealing a bit from an existing address.

`A$(I)=`, `INPUT A$(I)`, and `READ A$(I)` use the same checked store path.
`LEN(A$(I))` is the element length. Bare `LEN(A$)` returns the element count
when that string array is dimensioned, otherwise the scalar's length.
`LEN((A$))` explicitly measures the scalar even when the array exists.

## Host reference

The host stores text in Rust strings but models the same ordered allocation
blocks, free records, compaction boundaries, and temporary lengths as native
BASIC. Stored nonempty values cost length+1. Every temporary costs length+1.
Replacements need room for both the old allocation and the new allocation,
plus active temporaries. Both runners therefore reject the same capacity cases.

## Out of scope

`MID$`, `LEFT$`, `RIGHT$`, `ASC`, `CHR$`, `VAL`, and `INSTR` remain out of scope.
Program records remain 128 bytes and source input lines remain limited to
122 bytes. Build long strings with concatenation or supply up to 255 characters
to string INPUT; a literal in one source record cannot contain 255 characters.

## Tests

Keep the existing examples green, updating memory inspections for offsets and
length-prefixed records. Add room-name string arrays and verify reassignment
reclaims the old allocation. Cover 255-character values, rejection of a
256-character assignment without replacement, pool exhaustion and recovery,
free-block reuse/splitting, compaction of array offsets without touching integer
words, shared array capacity, INPUT/READ/LEN, repeated loops, and both runners.
