# RX-82 BASIC tokenized lines

Status: spec only. Program storage is still 256 records of 128 bytes at `1000`–`8FFF`: a line word, then NUL-terminated source. A zero line number is a free record.

This spec replaces that table with the Microsoft layout: a packed chain of tokenized lines in the same 32 KB. `LIST`, prompt `SAVE`, and the example files stay text. `LOAD` tokenizes as it reads. The host interpreter and the native ROM share one token table and one chain encoding, so a fixture can compare the bytes at `1000` as well as the output.

## Chain

The window `1000`–`8FFF` is no longer slotted. Lines are packed from `1000` upward. `0086` holds the address of the terminating pair. `0084` remains the execution cursor. `00B0` remains the array bump. An edit and a `DIM` must not share a pointer.

```text
1000  next   line   tokens...  00
      next   line   tokens...  00
      ...
0086  00 00                         end of program
8FFF  top of the window
```

`next` is a little-endian address, the byte after this line's terminating `00`. `line` is the line number, a little-endian word, never zero. The token stream is the bytes below. A line that would make `0086` pass `8FFF` is `? PROGRAM FULL`, and the old chain stays.

There are no holes. Replacing a line finds it by walking `next`, computes the size delta, and slides the tail with the string-copy loop. Deleting slides the tail down. Inserting slides it up. `0086` moves with the tail.

Direct mode does not write the chain. It tokenizes the input buffer at `0200` into the scratch at `0900` and runs that stream.

`10 PRINT "HI"` at the start of an empty program is:

```text
1000  0A 10          next = 100A
1002  0A 00          line 10
1004  90            PRINT
1005  B1 02 48 49   "HI"
1009  00            end of line
100A  00 00         end of program, 0086 = 100A
```

## Token byte

Bit 7 set means a token. Bit 7 clear means an ASCII character that is not part of a keyword, which is how punctuation survives: `+`, `-`, `*`, `/`, `(`, `)`, `,`, `=`, `<`, `>`.

```text
80 END      81 FOR      82 NEXT     83 DATA     84 INPUT
85 DIM      86 READ     87 LET      88 GOTO     89 RUN
8A IF       8B RESTORE  8C GOSUB    8D RETURN   8E REM
8F STOP     90 PRINT    91 LIST     92 NEW      93 SAVE
94 LOAD     95 QUIT     96 HELP     97 THEN     98 TO
99 STEP     9A LEN      9B PEEK     9C POKE     9D RND
9E RANDOMIZE
A0 MID$     A1 LEFT$    A2 RIGHT$   A3 CHR$     A4 ASC
A5 VAL      A6 INSTR
B0 number   B1 string   B2 line-ref B3 <>       B4 <=
B5 >=
```

`B0` is followed by a signed 16-bit little-endian word. `B1` is followed by a length byte and that many ASCII bytes, the same character rules as the pool. `B2` is followed by an unsigned line word, used by `GOTO`, `GOSUB`, `THEN`, and `RESTORE`. A stored line number is never a pointer. A pointer would rot on every insert. `REM` is followed by raw ASCII until the end byte, not tokens.

Keywords match only as a whole word. `NOTE` does not become `TO`. Case is folded at entry. `LIST` prints the canonical uppercase keyword.

One source line is one chain link. A `:` between statements is out of scope. A single line still has to fit in the remaining window. A line longer than the free tail is `? LINE TOO LONG`, and the old chain stays.

## Execution

The runner no longer lexes a stored line. It reads the token at `CD` and dispatches. The next line to run is the `next` pointer, not a search for the following line number. `GOTO`, `GOSUB`, and `RESTORE` walk from `1000` until the line number matches. Forty rooms do not make that walk matter. A line-start cache at `0900` can come later.

`PRINT` still accepts the `?` abbreviation on entry, and stores `90`. `LET` remains optional on entry. A comparison is the existing operator, except `<>`, `<=`, and `>=` are one token so the runner does not reassemble them.

`DATA` keeps its payload as text after the `DATA` token. `READ` already parses constants. Tokenizing the data would make `RESTORE` to a line harder to read and would not save a useful number of bytes.

## Editor and files

Entering `100 PRINT R$(0)` inserts or replaces that line number in order. Entering `100` alone deletes it and slides the tail down. `LIST` walks the chain and detokenizes. Prompt `SAVE` writes the detokenized source, one line per chain link, so the current `.bas` files remain the file format. `LOAD` sets `0086` to `1000`, writes the end marker, tokenizes each text line, and appends. A token error on load does not replace the program, which is the rule prompt `LOAD` already uses.

The host keeps the same chain in a byte buffer, not a map of 256 strings. It executes the tokens. Text exists only at `LIST` and `SAVE`.

## Tests

A one-line fixture stores `10 PRINT "HI"` and asserts `1000` is `0A 10 0A 00 90 B1 02 48 49 00 00 00`, with `0086` equal to `100A`. `LIST` prints `10 PRINT "HI"`. Inserting line 20 between 10 and 30 slides the tail up and rewrites the `next` word of line 10. Deleting line 20 slides it back. A line using `INSTR` and `MID$` round-trips through `SAVE` and `LOAD` on both runners. A program of 257 short lines loads. The same program in the old 128-byte slots would have stopped at line 256. Filling the window to `8FFF` is `? PROGRAM FULL` and leaves the previous chain in place.

## Out of scope

Several statements on one source line with `:`. A line-start cache. Saving the token image to disk. Bank switching. The chain is how MSX and Commodore got more program into the same RAM. This spec does that, and no more.
