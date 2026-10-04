# BASIC examples

Run these commands from the repository root. Every example works with both the
Rust reference interpreter and the native R8 BASIC ROM:

```sh
cargo run -p rx82 -- basic crates/rx82/examples/fibonacci.bas
cargo run -p rx82 -- basic --native crates/rx82/examples/fibonacci.bas
```

The native frontend also prints its banner and prompts. Program output should
otherwise agree.

| Program | What it exercises | Expected result |
| :--- | :--- | :--- |
| [countdown.bas](countdown.bas) | Negative `STEP` | 10, 8, 6, 4, 2, 0, then `Lift off!` |
| [squares.bas](squares.bas) | `INPUT`, multiplication, `FOR` | Input 5 produces squares 1, 4, 9, 16, 25 |
| [fibonacci.bas](fibonacci.bas) | Assignments, addition, loop state | Terms 0 through 15, ending at 610 |
| [factorials.bas](factorials.bas) | `GOSUB`, nested and zero-trip loops | 0! through 7!: 1, 1, 2, 6, 24, 120, 720, 5040 |
| [multiplication.bas](multiplication.bas) | Nested loops, tab-separated `PRINT` | A 6 by 6 multiplication table |
| [gcd.bas](gcd.bas) | Two inputs, integer division, conditionals, subroutines | Inputs 252 and 105 produce `GCD = 21` |

All examples use single-letter variables and signed 16-bit values. Squares fit
for inputs up to 181; larger inputs eventually overflow. The GCD example accepts
positive integers from 1 to 32767 and rejects nonpositive inputs.

For input-driven examples, enter values when prompted or pipe them:

```sh
printf '252\n105\n' | cargo run -p rx82 -- basic --native crates/rx82/examples/gcd.bas
```

## Inspect native execution

Load an example and pause before its BASIC statements execute:

```sh
cargo run -p rx82 -- basic --native --break-before-run crates/rx82/examples/factorials.bas
```

At the monitor prompt:

```text
m 1000
m 0300
g
m 0300
m 1380
q
```

Before `g`, the source is already in RAM and variables are still zero. `g`
executes the queued `RUN`, displays the program output, then returns to the
monitor when BASIC prompts for another command. `m` displays 128 bytes as hex
and ASCII. For factorials, the dumps after execution show:

- Variable F at `030A`: `B0 13`, little-endian `0x13B0` = **5040**.
- Variables I at `0310` and N at `031A`: `08 00`, both **8** after their loops.
- First program record at `1000`: `0A 00`, line **10**, followed by ASCII `REM`.
- Subroutine record at `1380`: `64 00`, line **100**, followed by its `REM` text.

With Fibonacci instead, variable A at `0300` ends as `DB 03` (**987**), B and C
as `3D 06` (**1597**), and I at `0310` as `10 00` (**16**). The last *printed*
term is 610; assignments advance the state once more before the loop exits.

Use `--step` instead of `--break-before-run` to stop at ROM entry (`C000`).
Then `B C000`, `G`, and `S` demonstrate stopping before an instruction and
stepping over it. `B` lists breakpoints, `BC C000` removes one, and `BC` clears
all. Breakpoints use hexadecimal machine-code addresses, not BASIC line numbers.

Guest output is displayed in monitor mode. For an input-driven example such as
`squares.bas`, use `G` to reach `INPUT`, `I 5` to queue an answer, then `G` to
continue. At BASIC's final prompt, `I LIST` then `G` lists its program without
leaving the monitor. `Q` exits; `I QUIT` followed by `G` halts the guest.

## Repeatable checks

```sh
cargo test -p rx82 --test basic_examples
```

These tests run the actual files through both interpreters, check their output
against expected results, and inspect the native source records and final
variables in RAM. Interactive examples receive scripted input.
