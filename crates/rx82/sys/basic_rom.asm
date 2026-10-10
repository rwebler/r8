; RX-82 native BASIC. All text parsing, editing and execution runs on R8.
; Nearby conditional targets use short branches; distant targets use inverted
; short branches over absolute JMPs. Recheck displacements when adding code.
; Devices: FF00..02 console; FF10..14 raw file byte stream.
; 0092 subroutine stack pointer, 0094 loop stack pointer.
; 00A6..AC file transfer state (LOAD validates before replacing the program).
; RAM: 0080 current line, 0082 running, 0084 next line pointer.
; 0200 INPUT buffer (256 bytes), 0300 variables (26 little-endian words).
; 0800 integer arrays, 0870 string arrays; 0340 scalar string offsets.
; 0900..0DFF token scratch, 0E00..0FFF string scratch; 8000..8FFF pool.
; Long lexical items spill from 0200..02FF to A000..A3FF during entry.
; 9000..9FFF integer array elements; 00B0 array allocation pointer.
; 1000..7FFF: packed next word, line word, token stream, zero byte.
; 0086 holds the final zero pair; stack grows down from BFFF.
    org 0xC100
BOOT:
    ld sp, 0xBFFF
    call NEW_PROGRAM
    call CLEAR_VARS
    call SC_RESET
    ld cd, BANNER
    call PUTS
PROMPT:
    call SC_PROMPT
    ld a, 0x00
    ld 0x03C2, a
    ld 0x03C3, a
    ld ab, 0x9000
    ld 0x00B6, b
    ld 0x00B7, a
    ld sp, 0xBFFF
    ld a, 0x00
    ld 0x0082, a
    ld cd, PROMPT_TEXT
    call PUTS
    call READ_TOKEN_LINE
    ld cd, 0x00E6
    ld b, (cd)
    ld a, (cd+0x01)
    cmp ab, 0x0000
    beq DIRECT
    call EDIT_PREPARED
    jmp PROMPT
DIRECT:
    ld gh, 0x0900
    call STATEMENT
    jmp PROMPT
; Command dispatch. MATCH advances GH only on success.
STATEMENT:
    call POOL_BEGIN
    call STATEMENT_BODY
    call STACK_RESET
    ret
STATEMENT_BODY:
    call SPACE
    cmp a, 0x00
    bne LONG_45
    ret
LONG_45:
    cmp a, 0x3F
    bne LONG_47
    jmp PRINT_SHORT
LONG_47:
VIDEO_DISPATCH_START:
    ld cd, KW_SCREEN
    call MATCH
    bne VIDEO_NEXT_SCREEN
    jmp VIDEO_SCREEN
VIDEO_NEXT_SCREEN:
    ld cd, KW_CLS
    call MATCH
    bne VIDEO_NEXT_CLS
    jmp VIDEO_CLS
VIDEO_NEXT_CLS:
    ld cd, KW_COLOR
    call MATCH
    bne VIDEO_NEXT_COLOR
    jmp VIDEO_COLOR
VIDEO_NEXT_COLOR:
    ld cd, KW_PLOT
    call MATCH
    bne VIDEO_NEXT_PLOT
    jmp VIDEO_PLOT
VIDEO_NEXT_PLOT:
    ld cd, KW_LINE
    call MATCH
    bne VIDEO_NEXT_LINE
    jmp VIDEO_LINE
VIDEO_NEXT_LINE:
VIDEO_DISPATCH_END:
    ld cd, KW_HELP
    call MATCH
    bne LONG_50
    jmp HELP
LONG_50:
    ld cd, KW_DIM
    call MATCH
    bne DIM_DISPATCH_NEXT
    jmp DIM
DIM_DISPATCH_NEXT:
    ld cd, KW_DEF
    call MATCH
    bne DEF_DISPATCH_NEXT
    jmp DEF_STATEMENT
DEF_DISPATCH_NEXT:
    ld cd, KW_DATA
    call MATCH
    bne DISPATCH_READ
    ret
DISPATCH_READ:
    ld cd, KW_READ
    call MATCH
    bne DISPATCH_RESTORE
    jmp READ_DATA
DISPATCH_RESTORE:
    ld cd, KW_RESTORE
    call MATCH
    bne DISPATCH_SAVE
    jmp RESTORE_DATA
DISPATCH_SAVE:
    ld cd, KW_POKE
    call MATCH
    bne DISPATCH_AFTER_POKE
    jmp POKE_BYTE
DISPATCH_AFTER_POKE:
    ld cd, KW_RANDOMIZE
    call MATCH
    bne DISPATCH_AFTER_RANDOMIZE
    jmp RANDOMIZE
DISPATCH_AFTER_RANDOMIZE:
    ld cd, KW_BSAVE
    call MATCH
    bne DISPATCH_BLOAD
    jmp BSAVE
DISPATCH_BLOAD:
    ld cd, KW_BLOAD
    call MATCH
    bne DISPATCH_TEXT_SAVE
    jmp BLOAD
DISPATCH_TEXT_SAVE:
    ld cd, KW_SAVE
    call MATCH
    bne LONG_53
    jmp SAVE
LONG_53:
    ld cd, KW_LOAD
    call MATCH
    bne LONG_56
    jmp LOAD
LONG_56:
    ld cd, KW_FOR
    call MATCH
    bne LONG_59
    jmp FOR
LONG_59:
    ld cd, KW_NEXT
    call MATCH
    bne LONG_62
    jmp NEXT
LONG_62:
    ld cd, KW_IF
    call MATCH
    bne LONG_65
    jmp IF
LONG_65:
    ld cd, KW_GOSUB
    call MATCH
    bne LONG_68
    jmp GOSUB
LONG_68:
    ld cd, KW_RETURN
    call MATCH
    bne LONG_71
    jmp RETURN
LONG_71:
    ld cd, KW_INPUT
    call MATCH
    bne LONG_74
    jmp INPUT
LONG_74:
    ld cd, KW_REM
    call MATCH
    beq DONE
LONG_77:
    ld cd, KW_PRINT
    call MATCH
    bne LONG_80
    jmp PRINT
LONG_80:
    ld cd, KW_LET
    call MATCH
    bne LONG_83
    jmp ASSIGN
LONG_83:
    ld cd, KW_GOTO
    call MATCH
    bne LONG_86
    jmp GOTO
LONG_86:
    ld cd, KW_END
    call MATCH
    beq END_RUN
LONG_89:
    ld cd, KW_STOP
    call MATCH
    beq END_RUN
LONG_92:
    ld cd, KW_RUN
    call MATCH
    bne LONG_95
    jmp RUN
LONG_95:
    ld cd, KW_LIST
    call MATCH
    bne LONG_98
    jmp LIST
LONG_98:
    ld cd, KW_NEW
    call MATCH
    beq NEW
LONG_101:
    ld cd, KW_QUIT
    call MATCH
    beq QUIT
LONG_104:
    jmp ASSIGN
DONE:
    ret
QUIT:
    call EOL
    halt
END_RUN:
    call EOL
    ld a, 0x00
    ld 0x0082, a
    ret
NEW:
    call REQUIRE_DIRECT
    call EOL
    call NEW_PROGRAM
    call CLEAR_VARS
    ret
NEW_PROGRAM:
    call FN_CLEAR_DIRECTORY
    ld cd, 0x1000
    ld 0x0086, d
    ld 0x0087, c
    ld (cd), 0x00
    ld a, 0x00
    ld (cd+0x01), a
    ret
CLEAR_VARS:
    ld cd, 0x0300
    ld a, 0x00
CLEAR_VAR:
    ld (cd), a
    inc cd
    cmp cd, 0x0334
    bne CLEAR_VAR
LONG_141:
    jmp CLEAR_ARRAYS
 ; Packed editor. E0 insertion, E2 old tail, E4 new size, E6 line.
EDIT_PREPARED:
    ld cd, 0x00E8
    ld b, (cd)
    ld a, (cd+0x01)
    ld cd, ef
    sec
    sub cd, ab
    ld ab, cd
    cmp ab, 0x0001
    bne EDIT_SIZE
    ld ab, 0xFFFC
EDIT_SIZE:
    clc
    add ab, 0x0004
    ld 0x00E4, b
    ld 0x00E5, a
    ld cd, 0x1000
EDIT_SCAN:
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, 0x0000
    beq EDIT_INSERT
    ld h, (cd+0x02)
    ld g, (cd+0x03)
    push cd
    ld cd, 0x00E6
    ld b, (cd)
    ld a, (cd+0x01)
    pop cd
    cmp gh, ab
    bcs EDIT_POSITION
    ld cd, ef
    jmp EDIT_SCAN
EDIT_POSITION:
    beq EDIT_REPLACE
EDIT_INSERT:
    ld ef, cd
EDIT_REPLACE:
    ld 0x00E0, d
    ld 0x00E1, c
    ld 0x00E2, f
    ld 0x00E3, e
    ld gh, 0x00E4
    ld b, (gh)
    ld a, (gh+0x01)
    clc
    add ab, cd
    sec
    sub ab, ef
    ld gh, 0x0086
    ld d, (gh)
    ld c, (gh+0x01)
    ld gh, cd
    clc
    add gh, ab
    cmp gh, 0x7FFF
    bcc EDIT_FITS
    jmp PROGRAM_FULL
EDIT_FITS:
    ld 0x0086, h
    ld 0x0087, g
    push ab
    ld ab, 0x00EC
    ld a, (ab)
    cmp a, 0x00
    beq EDIT_SHORT
    pop ab
    jmp EDIT_STAGED
EDIT_SHORT:
    pop ab
    cmp gh, cd
    bcc EDIT_DOWN
    beq EDIT_WRITE
    inc cd
    inc gh
EDIT_UP_LOOP:
    ld a, (cd)
    ld (gh), a
    cmp cd, ef
    beq EDIT_WRITE
    dec cd
    dec gh
    jmp EDIT_UP_LOOP
EDIT_DOWN:
    inc cd
    ld gh, ef
    clc
    add gh, ab
EDIT_DOWN_LOOP:
    ld a, (ef)
    ld (gh), a
    cmp ef, cd
    beq EDIT_WRITE
    inc ef
    inc gh
    jmp EDIT_DOWN_LOOP
EDIT_WRITE:
    ld cd, 0x00E0
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, 0x00E4
    ld b, (cd)
    ld a, (cd+0x01)
    cmp ab, 0x0000
    beq EDIT_LINKS
    push ab
    inc ef
    inc ef
    ld cd, 0x00E6
    ld a, (cd)
    ld (ef), a
    inc ef
    ld a, (cd+0x01)
    ld (ef), a
    inc ef
    ld cd, 0x0900
    pop ab
    sec
    sub ab, 0x0004
    ld gh, ab
EDIT_COPY:
    ld a, (cd)
    ld (ef), a
    inc cd
    inc ef
    dec gh
    bne EDIT_COPY
EDIT_LINKS:
    ld cd, 0x1000
    ld ef, 0x0086
    ld b, (ef)
    ld a, (ef+0x01)
EDIT_LINK_LOOP:
    cmp cd, ab
    beq EDIT_DONE
    ld gh, cd
    clc
    add gh, 0x0004
    push ab
    call TOKEN_LINE_END
    pop ab
    ld (cd), h
    ld (cd+0x01), g
    ld cd, gh
    jmp EDIT_LINK_LOOP
EDIT_DONE:
    call FN_CLEAR_DIRECTORY
    jmp RESET_DATA
FIND_NEXT:
    push gh
    ld cd, 0x1000
FIND_SCAN:
    ld h, (cd)
    ld g, (cd+0x01)
    cmp gh, 0x0000
    beq FIND_NONE
    ld f, (cd+0x02)
    ld e, (cd+0x03)
    cmp ef, ab
    bcc FIND_ADVANCE
    beq FIND_ADVANCE
    pop gh
    ret
FIND_ADVANCE:
    ld cd, gh
    jmp FIND_SCAN
FIND_NONE:
    ld cd, 0x0000
    pop gh
    ret
SEEK_AFTER:
    push ab
    ld cd, 0x0080
    ld b, (cd)
    ld a, (cd+0x01)
    call FIND_NEXT
    ld 0x0084, d
    ld 0x0085, c
    pop ab
    ret
LIST:
    call EOL
    ld ab, 0x0000
LIST_NEXT:
    call FIND_NEXT
    cmp cd, 0x0000
    bne LONG_236
    ret
LONG_236:
    ld ab, ef
    push ab
    push cd
    call PRINT_LINE
    ld a, 0x20
    call PUTCHAR
    pop gh
    clc
    add gh, 0x0004
    call DETOKENIZE
    call NEWLINE
    pop ab
    jmp LIST_NEXT
RUN:
    call EOL
    call CLEAR_VARS
    ld a, 0x01
    ld 0x0082, a
    call FN_BUILD_DIRECTORY
    ld ab, 0x0400
    ld 0x0092, b
    ld 0x0093, a
    ld ab, 0x0500
    ld 0x0094, b
    ld 0x0095, a
    ld ab, 0x0000
    ld 0x0080, b
    ld 0x0081, a
    ld a, 0x01
    ld 0x0082, a
    ld cd, 0x1000
    ld 0x0084, d
    ld 0x0085, c
RUN_NEXT:
    ld cd, 0x0084
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, 0x0000
    beq RUN_DONE
RUN_HEADER:
    ld cd, ef
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, 0x0000
    beq RUN_DONE
RUN_LINE:
    ld 0x0084, f
    ld 0x0085, e
    ld f, (cd+0x02)
    ld e, (cd+0x03)
    ld 0x0080, f
    ld 0x0081, e
    ld gh, cd
    clc
    add gh, 0x0004
    call STATEMENT
    ld cd, 0x0082
    ld a, (cd)
    cmp a, 0x00
    bne RUN_NEXT
LONG_280:
RUN_DONE:
    ld a, 0x00
    ld 0x0082, a
    ret
GOTO:
    ld a, 0x01
    ld 0x00A4, a
    call REQUIRE_RUN
    call TARGET
    push ab
    call EOL
    pop ab
GOTO_TARGET:
    cmp ab, 0x0000
    bne LONG_295
    jmp INVALID_LINE_NUMBER
LONG_295:
    ; Require an exact target, then set current to target minus one.
    dec ab
    push ab
    call FIND_NEXT
    cmp cd, 0x0000
    bne LONG_301
    jmp UNDEFINED_LINE
LONG_301:
    pop ab
    inc ab
    cmp ab, ef
    beq LONG_305
    jmp UNDEFINED_LINE
LONG_305:
    ld cd, 0x00A4
    ld c, (cd)
    cmp c, 0x00
    beq GOTO_STORE
LONG_309:
    call PRUNE_LOOPS
GOTO_STORE:
    dec ab
    ld 0x0080, b
    ld 0x0081, a
    jmp SEEK_AFTER
ASSIGN:
    call IS_STRING
    beq ASSIGN_INTEGER
    jmp ASSIGN_STRING
ASSIGN_INTEGER:
    call LOCATION
    push cd
    call SPACE
    cmp a, 0x3D
    beq LONG_321
    jmp EXPECTED_EQUALS
LONG_321:
    inc gh
    call EXPR
    push ab
    call EOL
    pop ab
    pop cd
    ld (cd), b
    ld (cd+0x01), a
    ret
PRINT:
    ld cd, KW_AT
    call MATCH
    bne PRINT_NORMAL
    jmp PRINT_AT
PRINT_NORMAL:
    call SPACE
    cmp a, 0x00
    bne LONG_334
    jmp NEWLINE
LONG_334:
PRINT_ITEM:
    call IS_STRING
    beq PRINT_VALUE
    call STRING_EXPRESSION
    call PRINT_STACK_STRING
    jmp PRINT_AFTER
PRINT_VALUE:
    call EXPR
    call PRINT_NUM
PRINT_AFTER:
    call SPACE
    cmp a, 0x3B
    beq PRINT_SEPARATOR
LONG_354:
    cmp a, 0x2C
    bne PRINT_END
LONG_356:
    ld a, 0x09
    call PUTCHAR
PRINT_SEPARATOR:
    inc gh
    call SPACE
    cmp a, 0x00
    bne LONG_363
    ret
LONG_363:
    jmp PRINT_ITEM
PRINT_END:
    call EOL
    jmp NEWLINE
PRINT_AT:
    ld cd, 0x00F3
    ld a, (cd)
    cmp a, 0x00
    beq PRINT_AT_SCREEN
    jmp PRINT_AT_ROUTE_ERROR
PRINT_AT_SCREEN:
    call EXPR
    cmp a, 0x80
    bcc PRINT_AT_ROW_NONNEGATIVE
    jmp PRINT_AT_RANGE_ERROR
PRINT_AT_ROW_NONNEGATIVE:
    cmp ab, 0x0018
    bcc PRINT_AT_ROW_VALID
    jmp PRINT_AT_RANGE_ERROR
PRINT_AT_ROW_VALID:
    ld 0x03D0, b
    ld 0x03D1, a
    call SPACE
    cmp a, 0x2C
    beq PRINT_AT_COMMA
    jmp EXPECTED_COMMA
PRINT_AT_COMMA:
    inc gh
    call EXPR
    cmp a, 0x80
    bcc PRINT_AT_COLUMN_NONNEGATIVE
    jmp PRINT_AT_RANGE_ERROR
PRINT_AT_COLUMN_NONNEGATIVE:
    cmp ab, 0x0028
    bcc PRINT_AT_COLUMN_VALID
    jmp PRINT_AT_RANGE_ERROR
PRINT_AT_COLUMN_VALID:
    ld 0x03CE, b
    ld 0x03CF, a
    call SPACE
    cmp a, 0x3B
    beq PRINT_AT_SEMICOLON
    jmp SYNTAX_ERROR
PRINT_AT_SEMICOLON:
    inc gh
    ld cd, 0x03D0
    ld b, (cd)
    ld a, (cd+0x01)
    ld ef, 0x0028
    call MUL_SIGNED
    ld ef, 0x03CE
    ld f, (ef)
    ld e, (ef+0x01)
    call ADD_SIGNED
    shl ab, 0x01
    ld 0x00F0, b
    ld 0x00F1, a
    ld cd, 0x03CE
    ld a, (cd)
    ld 0x00F2, a
    call SPACE
    cmp a, 0x00
    beq PRINT_AT_DONE
    jmp PRINT_NORMAL
PRINT_AT_DONE:
    ret
PRINT_AT_RANGE_ERROR:
    ld cd, PRINT_AT_RANGE_TEXT
    jmp REPORT_ERROR
PRINT_AT_RANGE_TEXT:
    data "? CURSOR OUT OF RANGE", 0x00
PRINT_AT_ROUTE_ERROR:
    ld cd, PRINT_AT_ROUTE_TEXT
    jmp REPORT_ERROR
PRINT_AT_ROUTE_TEXT:
    data "? SCREEN CONSOLE REQUIRED", 0x00
; Recursive-descent signed 16-bit expressions. GH is source; AB is result.
; CD/EF are preserved by expression functions. R8 stack holds intermediate values.
EXPR:
    push cd
    push ef
    call TERM
EXPR_MORE:
    push ab
    call SPACE
    ld c, a
    pop ab
    cmp c, 0x2B
    beq EXPR_OPERATOR
LONG_380:
    cmp c, 0x2D
    bne EXPR_DONE
LONG_382:
EXPR_OPERATOR:
    inc gh
    push ab
    push cd
    call TERM
    ld ef, ab
    pop cd
    pop ab
    cmp c, 0x2B
    beq EXPR_ADD
LONG_392:
    call SUB_SIGNED
    jmp EXPR_MORE
EXPR_ADD:
    call ADD_SIGNED
    jmp EXPR_MORE
EXPR_DONE:
    pop ef
    pop cd
    ret
TERM:
    push cd
    push ef
    call VALUE
TERM_MORE:
    push ab
    call SPACE
    ld c, a
    pop ab
    cmp c, 0x2A
    beq TERM_OPERATOR
LONG_412:
    cmp c, 0x2F
    bne TERM_DONE
LONG_414:
TERM_OPERATOR:
    inc gh
    push ab
    push cd
    call VALUE
    ld ef, ab
    pop cd
    pop ab
    cmp c, 0x2A
    beq TERM_MULTIPLY
LONG_424:
    call DIV_SIGNED
    jmp TERM_MORE
TERM_MULTIPLY:
    call MUL_SIGNED
    jmp TERM_MORE
TERM_DONE:
    pop ef
    pop cd
    ret
VALUE:
    ; Keep recursion from colliding with program storage.
    cmp sp, 0xA000
    bcs LONG_437
    jmp EXPRESSION_TOO_DEEP
LONG_437:
    call SPACE
    cmp a, 0x28
    bne LONG_440
    jmp VALUE_PAREN
LONG_440:
    cmp a, 0x2D
    bne LONG_442
    jmp VALUE_NEG
LONG_442:
    cmp a, 0x2B
    bne LONG_444
    jmp VALUE_PLUS
LONG_444:
    cmp a, 0xB0
    bne VALUE_NOT_TOKEN
    jmp NUMBER
VALUE_NOT_TOKEN:
    cmp a, 0xB2
    beq LONG_448
    cmp a, 0x30
    bcc VALUE_VAR
LONG_446:
    cmp a, 0x3A
    bcs VALUE_VAR
LONG_448:
    call NUMBER
    cmp ab, 0x8000
    bcc LONG_451
    jmp OVERFLOW
LONG_451:
    ret
VALUE_VAR:
    ld cd, KW_LEN
    call MATCH
    bne VALUE_NOT_LEN
    jmp LENGTH
VALUE_NOT_LEN:
    ld cd, KW_ASC
    call MATCH
    bne VALUE_NOT_ASC
    jmp STRING_ASC
VALUE_NOT_ASC:
    ld cd, KW_VAL
    call MATCH
    bne VALUE_NOT_VAL
    jmp STRING_VAL
VALUE_NOT_VAL:
    ld cd, KW_INSTR
    call MATCH
    bne VALUE_NOT_INSTR
    jmp STRING_INSTR
VALUE_NOT_INSTR:
    ld cd, KW_PEEK
    call MATCH
    bne VALUE_NOT_PEEK
    jmp PEEK_BYTE
VALUE_NOT_PEEK:
    ld cd, KW_RND
    call MATCH
    bne VALUE_NOT_RND
    jmp RANDOM_NUMBER
VALUE_NOT_RND:
    push gh
    call PEEK
    cmp a, 0x46
    bne VALUE_NOT_FN
    inc gh
    call PEEK
    cmp a, 0x4E
    bne VALUE_NOT_FN
    inc gh
    call PEEK
    cmp a, 0x41
    bcc VALUE_NOT_FN
    cmp a, 0x5B
    bcs VALUE_NOT_FN
    inc gh
    call SPACE
    cmp a, 0x28
    bne VALUE_NOT_FN
    pop gh
    jmp FN_CALL
VALUE_NOT_FN:
    pop gh
    call IS_STRING
    beq VALUE_INTEGER
    jmp TYPE_MISMATCH
VALUE_INTEGER:
    call LOCATION
    ld ef, 0x03C2
    ld b, (ef)
    ld a, (ef+0x01)
    cmp ab, cd
    bne VALUE_GLOBAL_INTEGER
    ld ef, 0x03C4
    ld b, (ef)
    ld a, (ef+0x01)
    ret
VALUE_GLOBAL_INTEGER:
    ld b, (cd)
    ld a, (cd+0x01)
    ret
VALUE_PLUS:
    inc gh
    jmp VALUE
VALUE_PAREN:
    inc gh
    call EXPR
    push ab
    call SPACE
    cmp a, 0x29
    beq LONG_467
    jmp EXPECTED_RPAREN
LONG_467:
    inc gh
    pop ab
    ret
VALUE_NEG:
    inc gh
    call SPACE
    cmp a, 0xB0
    beq VALUE_NEG_RECURSE
    cmp a, 0x30
    bcc VALUE_NEG_RECURSE
LONG_475:
    cmp a, 0x3A
    bcs VALUE_NEG_RECURSE
LONG_477:
    call NUMBER
    cmp ab, 0x8000
    beq NEGATE
LONG_480:
    bcc LONG_481
    jmp OVERFLOW
LONG_481:
    jmp NEGATE
VALUE_NEG_RECURSE:
    call VALUE
    cmp ab, 0x8000
    bne LONG_486
    jmp OVERFLOW
LONG_486:
    jmp NEGATE
; Two's complement, deliberately accepts 8000 for magnitude conversion.
NEGATE:
    push cd
    ld cd, 0x0000
    sec
    sub cd, ab
    ld ab, cd
    pop cd
    ret
ADD_SIGNED:
    push cd
    ld c, a
    and c, 0x80
    ld d, e
    and d, 0x80
    clc
    add ab, ef
    cmp c, d
    bne ADD_OK
LONG_506:
    ld d, a
    and d, 0x80
    cmp c, d
    beq LONG_510
    jmp OVERFLOW
LONG_510:
ADD_OK:
    pop cd
    ret
SUB_SIGNED:
    push cd
    ld c, a
    and c, 0x80
    ld d, e
    and d, 0x80
    sec
    sub ab, ef
    cmp c, d
    beq ADD_OK
LONG_523:
    ld d, a
    and d, 0x80
    cmp c, d
    beq LONG_527
    jmp OVERFLOW
LONG_527:
    pop cd
    ret
; Normalize AB, EF to unsigned magnitudes and store result sign at 0090.
MAGNITUDES:
    ld c, 0x00
    cmp a, 0x80
    bcc MAG_RIGHT
LONG_534:
    call NEGATE
    inc c
MAG_RIGHT:
    cmp e, 0x80
    bcc MAG_DONE
LONG_539:
    push ab
    ld ab, ef
    call NEGATE
    ld ef, ab
    pop ab
    inc c
MAG_DONE:
    and c, 0x01
    ld 0x0090, c
    ret
MAG_RESULT:
    ld cd, 0x0090
    ld c, (cd)
    cmp c, 0x00
    beq MAG_POSITIVE
LONG_554:
    cmp ab, 0x8000
    beq NEGATE
LONG_556:
    bcs OVERFLOW
LONG_557:
    jmp NEGATE
MAG_POSITIVE:
    cmp ab, 0x8000
    bcs OVERFLOW
LONG_561:
    ret
MUL_SIGNED:
    push cd
    call MAGNITUDES
    ld cd, 0x0000
    cmp ab, 0x0000
    beq MUL_DONE
MUL_LOOP:
    ; Consume one multiplier bit; LSR puts that bit in carry.
    lsr ab, 0x01
    bcc MUL_SHIFT
    clc
    add cd, ef
    bcs OVERFLOW
MUL_SHIFT:
    ; Do not double after the last bit: -32768 * 1 must remain valid.
    cmp ab, 0x0000
    beq MUL_DONE
    shl ef, 0x01
    bcc MUL_LOOP
    ; Remaining multiplier bits make an overflowing shift a real overflow.
    jmp OVERFLOW
MUL_DONE:
    ld ab, cd
    call MAG_RESULT
    pop cd
    ret
DIV_SIGNED:
    push cd
    cmp ef, 0x0000
    beq DIV_ZERO
LONG_583:
    call MAGNITUDES
    push gh
    ld cd, 0x0000
    ld gh, 0x0001
    cmp ab, ef
    bcc DIV_DONE
DIV_ALIGN:
    ; Magnitudes are at most 8000. Doubling a smaller divisor cannot wrap.
    cmp ef, ab
    bcs DIV_LOOP
    shl ef, 0x01
    shl gh, 0x01
    jmp DIV_ALIGN
DIV_LOOP:
    cmp ab, ef
    bcc DIV_SHIFT
    sec
    sub ab, ef
    or cd, gh
DIV_SHIFT:
    ; Descend through at most 16 quotient bits, retaining the remainder in AB.
    lsr ef, 0x01
    lsr gh, 0x01
    cmp gh, 0x0000
    bne DIV_LOOP
DIV_DONE:
    ld ab, cd
    call MAG_RESULT
    pop gh
    pop cd
    ret
OVERFLOW:
    ld cd, OVERFLOW_TEXT
    jmp REPORT_ERROR
DIV_ZERO:
    ld cd, DIV_ZERO_TEXT
    jmp REPORT_ERROR
OVERFLOW_TEXT:
    data "? INTEGER OVERFLOW", 0x00
DIV_ZERO_TEXT:
    data "? DIVISION BY ZERO", 0x00
VARIABLE:
    call SPACE
    cmp a, 0x41
    bcs LONG_611
    jmp EXPECTED_VARIABLE
LONG_611:
    cmp a, 0x5B
    bcc LONG_613
    jmp EXPECTED_VARIABLE
LONG_613:
    sec
    sub a, 0x41
    shl a, 0x01
    ld d, a
    ld c, 0x03
    inc gh
    ret
; Parse unsigned decimal to AB (0..65535); VALUE enforces signed limits.
NUMBER:
    ld a, (gh)
    cmp a, 0xB0
    beq NUMBER_TOKEN
    cmp a, 0xB2
    bne NUMBER_TEXT
NUMBER_TOKEN:
    inc gh
    ld b, (gh)
    inc gh
    ld a, (gh)
    inc gh
    ret
NUMBER_TEXT:
    push cd
    push ef
    ld ef, 0x0000
NUMBER_NEXT:
    call PEEK
    cmp a, 0x30
    bcc NUMBER_DONE
LONG_630:
    cmp a, 0x3A
    bcs NUMBER_DONE
LONG_632:
    sec
    sub a, 0x30
    ld b, a
    ld a, 0x00
    cmp ef, 0x1999
    bcc NUMBER_ACCUMULATE
LONG_638:
    bne OVERFLOW
LONG_639:
    cmp b, 0x06
    bcc LONG_641
    jmp OVERFLOW
LONG_641:
NUMBER_ACCUMULATE:
    ld cd, ef
    ; The precheck limits x to 6553, so 4*x fits without lost bits.
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
LONG_655:
    inc gh
    jmp NUMBER_NEXT
NUMBER_DONE:
    ld ab, ef
    pop ef
    pop cd
    ret
PRINT_NUM:
    push cd
    push ef
    push gh
    cmp a, 0x80
    bcc PRINT_POSITIVE
LONG_668:
    push ab
    ld a, 0x2D
    call PUTCHAR
    pop ab
    call NEGATE
PRINT_POSITIVE:
    ld ef, ab
    ld gh, 0x0000
    ld cd, 0x2710
    call PRINT_DIGIT
    ld cd, 0x03E8
    call PRINT_DIGIT
    ld cd, 0x0064
    call PRINT_DIGIT
    ld cd, 0x000A
    call PRINT_DIGIT
    ld a, f
    or a, 0x30
    call PUTCHAR
    pop gh
    pop ef
    pop cd
    ret
PRINT_DIGIT:
    ld b, 0x00
DIGIT_SUB:
    cmp ef, cd
    bcc DIGIT_END
LONG_697:
    sec
    sub ef, cd
    inc b
    jmp DIGIT_SUB
DIGIT_END:
    cmp b, 0x00
    bne DIGIT_EMIT
LONG_704:
    cmp gh, 0x0000
    bne LONG_706
    ret
LONG_706:
DIGIT_EMIT:
    ld gh, 0x0001
    ld a, b
    or a, 0x30
    call PUTCHAR
    ret
; PEEK uppercases ASCII letters but does not modify source or other registers.
PEEK:
    ld a, (gh)
    cmp a, 0x61
    bcc PEEK_DONE
LONG_718:
    cmp a, 0x7B
    bcs PEEK_DONE
LONG_720:
    sec
    sub a, 0x20
PEEK_DONE:
    ret
SPACE:
    call PEEK
    cmp a, 0x20
    beq SPACE_NEXT
LONG_728:
    cmp a, 0x09
    beq LONG_730
    ret
LONG_730:
SPACE_NEXT:
    inc gh
    jmp SPACE
EOL:
    call SPACE
    cmp a, 0x00
    beq LONG_737
    jmp SYNTAX_ERROR
LONG_737:
    ret
MATCH:
    ld a, (gh)
    cmp a, 0x80
    bcc MATCH_SOURCE
    ld a, (cd)
    push ef
    ld e, (gh)
    cmp a, e
    bne MATCH_TOKEN_DIFFERENT
    pop ef
    inc gh
    ld a, 0x00
    cmp a, 0x00
    ret
MATCH_TOKEN_DIFFERENT:
    pop ef
MATCH_TOKEN_FAIL:
    ld a, 0x01
    cmp a, 0x00
    ret
MATCH_SOURCE:
    inc cd
MATCH_TEXT:
    push gh
    push ef
MATCH_NEXT:
    ld e, (cd)
    cmp e, 0x00
    beq MATCH_BOUNDARY
LONG_745:
    call PEEK
    cmp a, e
    bne MATCH_FAIL
LONG_748:
    inc gh
    inc cd
    jmp MATCH_NEXT
MATCH_BOUNDARY:
    call WORD_CHAR
    bne MATCH_FAIL
    jmp MATCH_OK
MATCH_OK:
    pop ef
    pop cd
    ld a, 0x00
    cmp a, 0x00
    ret
MATCH_FAIL:
    pop ef
    pop gh
    ld a, 0x01
    cmp a, 0x00
    ret
READLINE:
    ld a, 0x00
    ld 0x00D0, a
    jmp READLINE_START
READLINE_STRING:
    ld a, 0x01
    ld 0x00D0, a
READLINE_START:
    call SC_EDIT_START
    ld a, 0x01
    ld 0x00EE, a
    ld gh, 0x0200
READLINE_NEXT:
    call GETCHAR
    cmp a, 0x0D
    beq READLINE_NEXT
LONG_775:
    cmp a, 0x0A
    beq READLINE_END
LONG_777:
    cmp a, 0xFF
    beq READLINE_FILE_END
LONG_779:
    cmp a, 0x09
    beq READLINE_STORE
LONG_781:
    cmp a, 0x20
    bcs LONG_783
    jmp BAD_INPUT_CHARACTER
LONG_783:
READLINE_STORE:
    push cd
    ld cd, 0x00D0
    ld b, (cd)
    pop cd
    cmp b, 0x00
    beq READLINE_SHORT_LIMIT
    cmp gh, 0x02FF
    bcc LONG_786
    ld cd, STRING_TOO_LONG_TEXT
    jmp DISCARD_INPUT_LINE
READLINE_SHORT_LIMIT:
    cmp gh, 0x02FF
    bcc LONG_786
    jmp INPUT_TOO_LONG
LONG_786:
    ld (gh), a
    inc gh
    jmp READLINE_NEXT
READLINE_FILE_END:
    ld cd, 0x00A7
    ld a, (cd)
    cmp a, 0x00
    bne LONG_794
    jmp BAD_INPUT_CHARACTER
LONG_794:
READLINE_END:
    ld a, 0x00
    ld 0x00EE, a
    ld (gh), 0x00
    ret
GETCHAR:
    jmp SC_GET
GETCHAR_RAW:
    push cd
    ld cd, 0x00A6
    ld a, (cd)
    cmp a, 0x00
    beq LONG_803
    jmp GETFILE
LONG_803:
GETCHAR_WAIT:
    ld cd, 0xFF00
    ld a, (cd)
    ld b, a
    and a, 0x01
    cmp a, 0x00
    bne GETCHAR_READY
LONG_810:
    and b, 0x02
    cmp b, 0x00
    bne QUIT_EOF
LONG_813:
    jmp GETCHAR_WAIT
GETCHAR_READY:
    inc cd
    ld a, (cd)
    pop cd
    ret
QUIT_EOF:
    halt
PUTCHAR:
    push cd
    push b
    ld cd, 0x00A8
    ld b, (cd)
    cmp b, 0x00
    beq PUTCHAR_CONSOLE
LONG_828:
    ld 0xFF13, a
    call FILE_CHECK
    jmp PUTCHAR_DONE
PUTCHAR_CONSOLE:
    call SC_OUTPUT
PUTCHAR_DONE:
    pop b
    pop cd
    ret
NEWLINE:
    ld a, 0x0A
    jmp PUTCHAR
PUTS:
    ld a, (cd)
    cmp a, 0x00
    bne LONG_844
    ret
LONG_844:
    call PUTCHAR
    inc cd
    jmp PUTS
SYNTAX_ERROR:
    ld cd, SYNTAX_ERROR_TEXT
REPORT_ERROR:
    push cd
    ld cd, 0x00EE
    ld a, (cd)
    cmp a, 0x00
    beq REPORT_DRAINED
REPORT_DRAIN:
    call GETCHAR
    cmp a, 0x0A
    beq REPORT_DRAINED
    cmp a, 0xFF
    bne REPORT_DRAIN
REPORT_DRAINED:
    ld a, 0x00
    ld 0x00EE, a
    pop cd
    ld sp, 0xBFFF
    ld a, 0x00
    ld 0x00A6, a
    ld 0x00A8, a
    ld a, 0x04
    ld 0xFF10, a
    call PUTS
    ld cd, 0x0082
    ld a, (cd)
    cmp a, 0x00
    beq ERROR_NEWLINE
LONG_861:
    ld cd, IN_LINE_TEXT
    call PUTS
    ld cd, 0x0080
    ld b, (cd)
    ld a, (cd+0x01)
    call PRINT_LINE
ERROR_NEWLINE:
    call NEWLINE
    jmp PROMPT
IN_LINE_TEXT:
    data " IN LINE ", 0x00
BANNER:
    data "RX-82 NATIVE BASIC", 0x0A, 0x00
PROMPT_TEXT:
    data "> ", 0x00
SYNTAX_ERROR_TEXT:
    data "? UNEXPECTED INPUT", 0x00

REQUIRE_RUN:
    push ab
    push cd
    ld cd, 0x0082
    ld a, (cd)
    cmp a, 0x00
    bne LONG_906
    jmp PROGRAM_ONLY
LONG_906:
    pop cd
    pop ab
    ret
; Signed comparison AB versus EF returns C=1 less, 2 equal, 4 greater.
COMPARE:
    cmp a, 0x80
    bcc COMP_LEFT_POS
LONG_913:
    cmp e, 0x80
    bcc COMP_LESS
LONG_915:
    jmp COMP_UNSIGNED
COMP_LEFT_POS:
    cmp e, 0x80
    bcs COMP_GREATER
LONG_919:
COMP_UNSIGNED:
    cmp ab, ef
    bcc COMP_LESS
LONG_922:
    beq COMP_EQUAL
LONG_923:
COMP_GREATER:
    ld c, 0x04
    ret
COMP_LESS:
    ld c, 0x01
    ret
COMP_EQUAL:
    ld c, 0x02
    ret
IF:
    call IS_STRING
    beq IF_INTEGER_LEFT
    call STRING_EXPRESSION
    ld a, 0x01
    ld 0x00BF, a
    jmp IF_LEFT_READY
IF_INTEGER_LEFT:
    call EXPR
    push ab
    ld a, 0x00
    ld 0x00BF, a
    pop ab
IF_LEFT_READY:
    push ab
    call SPACE
    ld b, 0x05
    cmp a, 0xB3
    beq IF_OPERATOR_DONE
    ld b, 0x03
    cmp a, 0xB4
    beq IF_OPERATOR_DONE
    ld b, 0x06
    cmp a, 0xB5
    beq IF_OPERATOR_DONE
    ld b, 0x02
    cmp a, 0x3D
    beq IF_OPERATOR_DONE
LONG_939:
    ld b, 0x01
    cmp a, 0x3C
    beq IF_LESS_OP
LONG_942:
    ld b, 0x04
    cmp a, 0x3E
    beq LONG_945
    jmp EXPECTED_COMPARISON
LONG_945:
    inc gh
    call PEEK
    cmp a, 0x3D
    bne IF_RIGHT
LONG_949:
    ld b, 0x06
    jmp IF_OPERATOR_DONE
IF_LESS_OP:
    inc gh
    call PEEK
    cmp a, 0x3D
    beq IF_LE_OP
LONG_956:
    cmp a, 0x3E
    bne IF_RIGHT
LONG_958:
    ld b, 0x05
    jmp IF_OPERATOR_DONE
IF_LE_OP:
    ld b, 0x03
IF_OPERATOR_DONE:
    inc gh
IF_RIGHT:
    push b
    ld cd, 0x00BF
    ld a, (cd)
    cmp a, 0x00
    beq IF_INTEGER_RIGHT
    call STRING_EXPRESSION
    call COMPARE_STACK_STRINGS
    pop d
    pop ab
    jmp IF_COMPARED
IF_INTEGER_RIGHT:
    call EXPR
    ld ef, ab
    pop d
    pop ab
    call COMPARE
IF_COMPARED:
    push cd
    call SPACE
    ld cd, KW_THEN
    call MATCH
    beq LONG_976
    jmp EXPECTED_THEN
LONG_976:
    pop cd
    cmp c, d
    beq IF_TRUE
LONG_979:
    cmp d, 0x03
    bne IF_GE_TEST
LONG_981:
    cmp c, 0x04
    bne IF_TRUE
LONG_983:
    ret
IF_GE_TEST:
    cmp d, 0x06
    bne IF_NE_TEST
LONG_987:
    cmp c, 0x01
    bne IF_TRUE
LONG_989:
    ret
IF_NE_TEST:
    cmp d, 0x05
    beq LONG_993
    ret
LONG_993:
    cmp c, 0x02
    bne LONG_995
    ret
LONG_995:
IF_TRUE:
    call SPACE
    cmp a, 0xB2
    beq LONG_1001
    cmp a, 0x30
    bcc IF_STATEMENT
LONG_999:
    cmp a, 0x3A
    bcs IF_STATEMENT
LONG_1001:
    jmp GOTO
IF_STATEMENT:
    ld cd, KW_FOR
    call MATCH
    bne LONG_1006
    jmp STANDALONE_LOOP
LONG_1006:
    ld cd, KW_NEXT
    call MATCH
    bne LONG_1009
    jmp STANDALONE_LOOP
LONG_1009:
    jmp STATEMENT
GOSUB:
    ld a, 0x00
    ld 0x00A4, a
    call REQUIRE_RUN
    call TARGET
    push ab
    call EOL
    ld cd, 0x0092
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, 0x0500
    bcc LONG_1022
    jmp GOSUB_STACK_FULL
LONG_1022:
    ld cd, 0x0080
    ld b, (cd)
    ld a, (cd+0x01)
    ld (ef), b
    inc ef
    ld (ef), a
    inc ef
    ld cd, 0x0094
    ld b, (cd)
    ld a, (cd+0x01)
    ld (ef), b
    inc ef
    ld (ef), a
    inc ef
    ld 0x0092, f
    ld 0x0093, e
    pop ab
    jmp GOTO_TARGET
RETURN:
    call REQUIRE_RUN
    call EOL
    ld cd, 0x0092
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, 0x0400
    bne LONG_1048
    jmp RETURN_WITHOUT_GOSUB
LONG_1048:
    dec ef
    ld a, (ef)
    ld 0x0095, a
    dec ef
    ld a, (ef)
    ld 0x0094, a
    dec ef
    ld a, (ef)
    ld 0x0081, a
    dec ef
    ld a, (ef)
    ld 0x0080, a
    ld 0x0092, f
    ld 0x0093, e
    jmp SEEK_AFTER
INPUT:
    call IS_STRING
    beq INPUT_INTEGER
    jmp INPUT_STRING
INPUT_INTEGER:
    call SC_INPUT_BEGIN
    call LOCATION
    push cd
    call EOL
    ld a, 0x3F
    call PUTCHAR
    ld a, 0x20
    call PUTCHAR
    call READLINE
    ld gh, 0x0200
    call EXPR
    push ab
    call EOL
    pop ab
    pop cd
    ld (cd), b
    ld (cd+0x01), a
    call SC_INPUT_END
    ret
; A lone unsigned literal can address any line. Expressions are signed 16-bit.
TARGET:
    call SPACE
    push gh
    cmp a, 0xB2
    beq LONG_1089
TARGET_SOURCE:
    cmp a, 0x30
    bcc TARGET_EXPR
LONG_1087:
    cmp a, 0x3A
    bcs TARGET_EXPR
LONG_1089:
    call NUMBER
    push ab
    call SPACE
    ld e, a
    pop ab
    cmp e, 0x00
    bne TARGET_EXPR
LONG_1096:
    pop cd
    ret
TARGET_EXPR:
    pop gh
    jmp EXPR

; Loop frames at 0500..07FF, 16 bytes each:
; +0 FOR line, +2 NEXT line, +4 variable address, +6 limit, +8 step.
; 0094 points immediately past the active frames. Scratch 0098..00A5.
FOR:
    call REQUIRE_RUN
    call IS_STRING
    beq FOR_INTEGER_VARIABLE
    jmp TYPE_MISMATCH
FOR_INTEGER_VARIABLE:
    call VARIABLE
    push cd
    call SPACE
    cmp a, 0x3D
    beq LONG_1122
    jmp EXPECTED_EQUALS
LONG_1122:
    inc gh
    call EXPR
    push ab
    call SPACE
    ld cd, KW_TO
    call MATCH
    beq LONG_1129
    jmp EXPECTED_TO
LONG_1129:
    call EXPR
    push ab
    call SPACE
    ld cd, KW_STEP
    call MATCH
    bne FOR_DEFAULT_STEP
LONG_1135:
    call EXPR
    jmp FOR_STEP_READY
FOR_DEFAULT_STEP:
    ld ab, 0x0001
FOR_STEP_READY:
    cmp ab, 0x0000
    bne LONG_1142
    jmp ZERO_STEP
LONG_1142:
    push ab
    call EOL
    pop ab
    ld 0x009E, b
    ld 0x009F, a
    pop ab
    ld 0x009C, b
    ld 0x009D, a
    pop ab
    ld 0x009A, b
    ld 0x009B, a
    pop cd
    ld 0x0098, d
    ld 0x0099, c
    ; Reject reuse of an active loop variable.
    ld gh, 0x0094
    ld f, (gh)
    ld e, (gh+0x01)
    ld gh, 0x0500
FOR_ACTIVE_SCAN:
    cmp gh, ef
    beq FOR_FIND_END
LONG_1164:
    ld b, (gh+0x04)
    ld a, (gh+0x05)
    cmp ab, cd
    bne LONG_1168
    jmp LOOP_VARIABLE_ACTIVE
LONG_1168:
    clc
    add gh, 0x0010
    jmp FOR_ACTIVE_SCAN
FOR_FIND_END:
    ld cd, 0x0080
    ld b, (cd)
    ld a, (cd+0x01)
    ld c, 0x01
    ld 0x0096, c
FOR_SCAN_NEXT:
    call FIND_NEXT
    cmp cd, 0x0000
    bne LONG_1181
    jmp FOR_WITHOUT_NEXT
LONG_1181:
    ld 0x00A0, f
    ld 0x00A1, e
    ld gh, cd
    clc
    add gh, 0x0004
    call SPACE
    ld cd, KW_FOR
    call MATCH
    bne FOR_SCAN_CLOSE
LONG_1190:
    inc (0x0096)
    jmp FOR_SCAN_ADVANCE
FOR_SCAN_CLOSE:
    ld cd, KW_NEXT
    call MATCH
    bne FOR_SCAN_ADVANCE
LONG_1196:
    dec (0x0096)
    beq FOR_MATCHED
LONG_1198:
FOR_SCAN_ADVANCE:
    ld cd, 0x00A0
    ld b, (cd)
    ld a, (cd+0x01)
    jmp FOR_SCAN_NEXT
FOR_MATCHED:
    call SPACE
    cmp a, 0x00
    beq FOR_SET
LONG_1207:
    call VARIABLE
    push cd
    call EOL
    pop ef
    ld cd, 0x0098
    ld b, (cd)
    ld a, (cd+0x01)
    cmp ab, ef
    beq LONG_1216
    jmp NEXT_MISMATCH
LONG_1216:
FOR_SET:
    ld cd, 0x0098
    ld f, (cd)
    ld e, (cd+0x01)
    ld b, (cd+0x02)
    ld a, (cd+0x03)
    ld (ef), b
    ld (ef+0x01), a
    ld f, (cd+0x04)
    ld e, (cd+0x05)
    call COMPARE
    ld b, c
    ld cd, 0x009F
    ld a, (cd)
    cmp a, 0x80
    bcs FOR_NEGATIVE
LONG_1232:
    cmp b, 0x04
    beq FOR_SKIP
LONG_1234:
    jmp FOR_PUSH
FOR_NEGATIVE:
    cmp b, 0x01
    beq FOR_SKIP
LONG_1238:
FOR_PUSH:
    ld cd, 0x0094
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, 0x0800
    bcc LONG_1244
    jmp FOR_STACK_FULL
LONG_1244:
    ld cd, 0x0080
    ld b, (cd)
    ld a, (cd+0x01)
    ld (ef), b
    ld (ef+0x01), a
    ld cd, 0x00A0
    ld b, (cd)
    ld a, (cd+0x01)
    ld (ef+0x02), b
    ld (ef+0x03), a
    ld cd, 0x0098
    ld b, (cd)
    ld a, (cd+0x01)
    ld (ef+0x04), b
    ld (ef+0x05), a
    ld b, (cd+0x04)
    ld a, (cd+0x05)
    ld (ef+0x06), b
    ld (ef+0x07), a
    ld b, (cd+0x06)
    ld a, (cd+0x07)
    ld (ef+0x08), b
    ld (ef+0x09), a
    clc
    add ef, 0x0010
    ld 0x0094, f
    ld 0x0095, e
    ret
FOR_SKIP:
    ld cd, 0x00A0
    ld b, (cd)
    ld a, (cd+0x01)
    ld 0x0080, b
    ld 0x0081, a
    jmp SEEK_AFTER
; CD = current call's loop base (0500 outside GOSUB).
LOOP_BASE:
    ld cd, 0x0092
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, 0x0500
    cmp ef, 0x0400
    bne LONG_1287
    ret
LONG_1287:
    sec
    sub ef, 0x0002
    ld d, (ef)
    ld c, (ef+0x01)
    ret
NEXT:
    call REQUIRE_RUN
    call LOOP_BASE
    ld ef, 0x0094
    ld b, (ef)
    ld a, (ef+0x01)
    cmp ab, cd
    bne LONG_1300
    jmp NEXT_WITHOUT_FOR
LONG_1300:
    sec
    sub ab, 0x0010
    ld 0x00A2, b
    ld 0x00A3, a
    call SPACE
    cmp a, 0x00
    beq NEXT_MATCH
LONG_1307:
    call VARIABLE
    push cd
    call EOL
    pop ef
    ld cd, 0x00A2
    ld b, (cd)
    ld a, (cd+0x01)
    ld cd, ab
    ld b, (cd+0x04)
    ld a, (cd+0x05)
    cmp ab, ef
    beq LONG_1319
    jmp NEXT_MISMATCH
LONG_1319:
NEXT_MATCH:
    ld cd, 0x00A2
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, 0x0080
    ld b, (cd)
    ld a, (cd+0x01)
    ld d, (ef+0x02)
    ld c, (ef+0x03)
    cmp ab, cd
    beq LONG_1330
    jmp NEXT_MISMATCH
LONG_1330:
    ld d, (ef+0x04)
    ld c, (ef+0x05)
    ld b, (cd)
    ld a, (cd+0x01)
    push cd
    ld cd, ef
    ld f, (cd+0x08)
    ld e, (cd+0x09)
    call ADD_SIGNED
    pop cd
    ld (cd), b
    ld (cd+0x01), a
    ld cd, 0x00A2
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, ef
    ld f, (cd+0x06)
    ld e, (cd+0x07)
    push cd
    call COMPARE
    ld d, c
    pop ef
    ld a, (ef+0x09)
    cmp a, 0x80
    bcs NEXT_NEGATIVE
LONG_1355:
    cmp d, 0x04
    beq NEXT_POP
LONG_1357:
    jmp NEXT_REPEAT
NEXT_NEGATIVE:
    cmp d, 0x01
    beq NEXT_POP
LONG_1361:
NEXT_REPEAT:
    ld b, (ef)
    ld a, (ef+0x01)
    ld 0x0080, b
    ld 0x0081, a
    jmp SEEK_AFTER
NEXT_POP:
    ld 0x0094, f
    ld 0x0095, e
    ret

; AB is a jump target. Discard loops exited in the current subroutine only.
PRUNE_LOOPS:
    push gh
    call LOOP_BASE
    ld gh, cd
PRUNE_AGAIN:
    ld cd, 0x0094
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, gh
    beq PRUNE_DONE
LONG_1391:
    sec
    sub ef, 0x0010
    ld d, (ef)
    ld c, (ef+0x01)
    cmp ab, cd
    bcc PRUNE_POP
LONG_1397:
    beq PRUNE_POP
LONG_1398:
    ld d, (ef+0x02)
    ld c, (ef+0x03)
    cmp ab, cd
    bcc PRUNE_DONE
LONG_1402:
    beq PRUNE_DONE
LONG_1403:
PRUNE_POP:
    ld 0x0094, f
    ld 0x0095, e
    jmp PRUNE_AGAIN
PRUNE_DONE:
    pop gh
    ret
PRINT_LINE:
    push cd
    push ef
    push gh
    jmp PRINT_POSITIVE

; Host file port is a raw byte stream. Filename parsing, validation, program
; serialization and line insertion below all execute on the R8 CPU.
; 00A6 file input flag; 00A7 EOF; 00A8 file output flag;
; 00AA validation line count; 00AC load pass (0 validate, 1 install).
REQUIRE_DIRECT:
    ld cd, 0x0082
    ld a, (cd)
    cmp a, 0x00
    beq LONG_1425
    jmp DIRECT_ONLY
LONG_1425:
    ret
FILENAME:
    call REQUIRE_DIRECT
    call FILENAME_LITERAL
    call EOL
    ret
FILENAME_LITERAL:
    call SPACE
    cmp a, 0xB1
    bne FILENAME_TEXT
    inc gh
    ld b, (gh)
    inc gh
    cmp b, 0x00
    bne FILENAME_TOKEN_VALID
    jmp EXPECTED_FILENAME
FILENAME_TOKEN_VALID:
    ld a, 0x00
    ld 0xFF14, a
FILENAME_TOKEN_BYTE:
    cmp b, 0x00
    beq FILENAME_VALID
    ld a, (gh)
    ld 0xFF12, a
    call FILE_CHECK
    inc gh
    dec b
    jmp FILENAME_TOKEN_BYTE
FILENAME_TEXT:
    cmp a, 0x22
    beq LONG_1431
    jmp EXPECTED_FILENAME
LONG_1431:
    inc gh
    ld b, 0x00
    ld a, 0x00
    ld 0xFF14, a
FILENAME_CHAR:
    ld a, (gh)
    inc gh
    cmp a, 0x00
    bne LONG_1439
    jmp UNTERMINATED_STRING
LONG_1439:
    cmp a, 0x22
    beq FILENAME_END
LONG_1441:
    inc b
    ld 0xFF12, a
    call FILE_CHECK
    jmp FILENAME_CHAR
FILENAME_END:
    cmp b, 0x00
    bne FILENAME_VALID
    jmp EXPECTED_FILENAME
FILENAME_VALID:
    ret
FILE_CHECK:
    push ab
    push cd
    ld cd, 0xFF11
    ld a, (cd)
    and a, 0x80
    cmp a, 0x00
    beq LONG_1455
    jmp FILE_ERROR
LONG_1455:
    pop cd
    pop ab
    ret
; Binary transfers use 00D5/6 for the current address, 00D7/8 for
; remaining/count, and 00D9 for the exclusive region-end high byte.
; These bytes are lexical scratch after a line has been tokenized.
BINARY_ARGS:
    call FILENAME_LITERAL
    call BINARY_COMMA
    call MEM_ADDRESS
    ld 0x00D5, b
    ld 0x00D6, a
    cmp ab, 0xC000
    bcc BINARY_MAIN_RAM
    jmp BINARY_BAD_RANGE
BINARY_MAIN_RAM:
    ld a, 0xC0
    ld 0x00D9, a
    ret
BINARY_COMMA:
    call SPACE
    cmp a, 0x2C
    beq BINARY_COMMA_OK
    jmp EXPECTED_COMMA
BINARY_COMMA_OK:
    inc gh
    ret
BINARY_BAD_RANGE:
    ld cd, BINARY_RANGE_TEXT
    jmp REPORT_ERROR
BINARY_RANGE_TEXT:
    data "? BINARY RANGE OUTSIDE RAM", 0x00
BINARY_BAD_LENGTH:
    ld cd, BINARY_LENGTH_TEXT
    jmp REPORT_ERROR
BINARY_LENGTH_TEXT:
    data "? INVALID BINARY LENGTH", 0x00
; The address parser accepts unsigned literals; length accepts the same
; syntax only when the literal is the complete remaining argument.
BINARY_LENGTH:
    call SPACE
    push gh
    cmp a, 0xB0
    beq BINARY_LENGTH_LITERAL
    cmp a, 0xB2
    beq BINARY_LENGTH_LITERAL
    cmp a, 0x30
    bcc BINARY_LENGTH_EXPR
    cmp a, 0x3A
    bcs BINARY_LENGTH_EXPR
BINARY_LENGTH_LITERAL:
    ld 0x00A9, a
    call NUMBER
    ld cd, 0x00A9
    ld c, (cd)
    cmp c, 0xB0
    bne BINARY_LENGTH_LITERAL_OK
    cmp a, 0x80
    bcs BINARY_BAD_LENGTH
BINARY_LENGTH_LITERAL_OK:
    push ab
    call SPACE
    ld c, a
    pop ab
    cmp c, 0x00
    beq BINARY_LENGTH_DONE
BINARY_LENGTH_EXPR:
    pop gh
    call EXPR
    cmp a, 0x80
    bcs BINARY_BAD_LENGTH
BINARY_LENGTH_OK:
    ret
BINARY_LENGTH_DONE:
    pop cd
    ret
BSAVE:
    call BINARY_ARGS
    call BINARY_COMMA
    call BINARY_LENGTH
    ld 0x00D7, b
    ld 0x00D8, a
    call EOL
    ld cd, 0x00D5
    ld b, (cd)
    ld a, (cd+0x01)
    ld cd, 0x00D7
    ld f, (cd)
    ld e, (cd+0x01)
    clc
    add ab, ef
    bcc BSAVE_NO_WRAP
    jmp BINARY_BAD_RANGE
BSAVE_NO_WRAP:
    ld cd, 0x00D9
    ld c, (cd)
    cmp c, 0xC0
    beq BSAVE_MAIN_CHECK
    cmp ab, 0xFF00
    jmp BSAVE_RANGE_RESULT
BSAVE_MAIN_CHECK:
    cmp ab, 0xC000
BSAVE_RANGE_RESULT:
    bcc BSAVE_OPEN
    beq BSAVE_OPEN
    jmp BINARY_BAD_RANGE
BSAVE_OPEN:
    push gh
    ld a, 0x02
    ld 0xFF10, a
    call FILE_CHECK
BSAVE_BYTE:
    ld cd, 0x00D7
    ld b, (cd)
    ld a, (cd+0x01)
    cmp ab, 0x0000
    beq BINARY_CLOSE
BSAVE_BYTE_MORE:
    dec ab
    ld (cd), b
    ld (cd+0x01), a
    ld cd, 0x00D5
    ld h, (cd)
    ld g, (cd+0x01)
    ld a, (gh)
    ld 0xFF13, a
    call FILE_CHECK
    inc gh
    ld (cd), h
    ld (cd+0x01), g
    jmp BSAVE_BYTE
BINARY_CLOSE:
    ld a, 0x03
    ld 0xFF10, a
    call FILE_CHECK
    pop gh
    ret
BLOAD:
    call BINARY_ARGS
    call EOL
    push gh
    ld a, 0x01
    ld 0xFF10, a
    call FILE_CHECK
    ld a, 0x00
    ld 0x00D7, a
    ld 0x00D8, a
BLOAD_SCAN:
    call FILE_CHECK
    ld cd, 0xFF11
    ld a, (cd)
    and a, 0x02
    cmp a, 0x00
    bne BLOAD_REWIND
BLOAD_SCAN_MORE:
    ld cd, 0x00D5
    ld b, (cd)
    ld a, (cd+0x01)
    ld cd, 0x00D9
    ld c, (cd)
    cmp c, 0xC0
    beq BLOAD_SCAN_MAIN_CHECK
    cmp ab, 0xFF00
    jmp BLOAD_SCAN_RANGE_RESULT
BLOAD_SCAN_MAIN_CHECK:
    cmp ab, 0xC000
BLOAD_SCAN_RANGE_RESULT:
    bcc BLOAD_SCAN_FITS
    jmp BINARY_BAD_RANGE
BLOAD_SCAN_FITS:
    inc ab
    ld 0x00D5, b
    ld 0x00D6, a
    ld cd, 0x00D7
    ld b, (cd)
    ld a, (cd+0x01)
    inc ab
    ld (cd), b
    ld (cd+0x01), a
    ld cd, 0xFF13
    ld a, (cd)
    call FILE_CHECK
    jmp BLOAD_SCAN
BLOAD_REWIND:
    ld a, 0x05
    ld 0xFF10, a
    call FILE_CHECK
    ; Recompute the first address from the final pointer and byte count.
    ld cd, 0x00D5
    ld b, (cd)
    ld a, (cd+0x01)
    ld cd, 0x00D7
    ld f, (cd)
    ld e, (cd+0x01)
    sec
    sub ab, ef
    ld 0x00D5, b
    ld 0x00D6, a
BLOAD_BYTE:
    ld cd, 0x00D7
    ld b, (cd)
    ld a, (cd+0x01)
    cmp ab, 0x0000
    bne BLOAD_BYTE_MORE
    jmp BINARY_CLOSE
BLOAD_BYTE_MORE:
    dec ab
    ld (cd), b
    ld (cd+0x01), a
    ld cd, 0xFF13
    ld a, (cd)
    call FILE_CHECK
    ld cd, 0x00D5
    ld h, (cd)
    ld g, (cd+0x01)
    ld (gh), a
    inc gh
    ld (cd), h
    ld (cd+0x01), g
    jmp BLOAD_BYTE
SAVE:
    call FILENAME
    ld a, 0x02
    ld 0xFF10, a
    call FILE_CHECK
    ld a, 0x01
    ld 0x00A8, a
    call LIST
    ld a, 0x00
    ld 0x00A8, a
    ld a, 0x03
    ld 0xFF10, a
    call FILE_CHECK
    ret
LOAD:
    call FILENAME
    ld a, 0x01
    ld 0xFF10, a
    call FILE_CHECK
    ld a, 0x00
    ld 0x00A7, a
    ld 0x00AA, a
    ld 0x00AB, a
    ld 0x00AC, a
    ld a, 0x01
    ld 0x00A6, a
LOAD_LINE:
    ld cd, 0x00A7
    ld a, (cd)
    cmp a, 0x00
    bne LOAD_EOF
LONG_1489:
    call READ_TOKEN_LINE
    ld cd, 0x00E6
    ld b, (cd)
    ld a, (cd+0x01)
    cmp ab, 0x0000
    bne LOAD_NUMBERED
    cmp ef, 0x0901
    beq LOAD_LINE
    jmp INVALID_LINE_NUMBER
LOAD_NUMBERED:
    ld cd, 0x00AC
    ld c, (cd)
    cmp c, 0x00
    bne LOAD_INSTALL
LONG_1508:
    cmp ef, 0x0901
    beq LOAD_LINE
LOAD_HAS_BODY:
    ld ab, ef
    sec
    sub ab, 0x0900
    clc
    add ab, 0x0004
    ld cd, 0x00AA
    ld f, (cd)
    ld e, (cd+0x01)
    clc
    add ab, ef
    bcc LOAD_SIZE_NO_WRAP
    jmp PROGRAM_FULL
LOAD_SIZE_NO_WRAP:
    cmp ab, 0x7FFF
    bcc LOAD_SIZE_OK
    jmp PROGRAM_FULL
LOAD_SIZE_OK:
    ld (cd), b
    ld (cd+0x01), a
    jmp LOAD_LINE
LOAD_INSTALL:
    call EDIT_PREPARED
    jmp LOAD_LINE
LOAD_EOF:
    ld cd, 0x00AC
    ld a, (cd)
    cmp a, 0x00
    bne LOAD_DONE
LONG_1525:
    ; Rewind the immutable byte snapshot, then install. Validation has not
    ; touched the program chain or variables. The second pass cannot exceed RAM.
    ld a, 0x05
    ld 0xFF10, a
    call FILE_CHECK
    call NEW_PROGRAM
    call CLEAR_VARS
    ld a, 0x01
    ld 0x00AC, a
    ld a, 0x00
    ld 0x00A7, a
    jmp LOAD_LINE
LOAD_DONE:
    ld a, 0x00
    ld 0x00A6, a
    ld a, 0x03
    ld 0xFF10, a
    call FILE_CHECK
    ret
GETFILE:
    call FILE_CHECK
    ld cd, 0xFF11
    ld a, (cd)
    and a, 0x02
    cmp a, 0x00
    bne GETFILE_EOF
LONG_1551:
    ld cd, 0xFF13
    ld a, (cd)
    pop cd
    ret
GETFILE_EOF:
    ld a, 0x01
    ld 0x00A7, a
    ld a, 0xFF
    pop cd
    ret
FILE_ERROR:
    ld cd, FILE_ERROR_TEXT
    jmp REPORT_ERROR
FILE_ERROR_TEXT:
    data "? FILE ERROR", 0x00

PRINT_SHORT:
    inc gh
    jmp PRINT
HELP:
    call EOL
    ld cd, HELP_TEXT
    jmp PUTS
HELP_TEXT:
VIDEO_HELP_START:
    data "SCREEN 0/1 CLS COLOR ink,paper PLOT x,y LINE x1,y1,x2,y2", 0x0A
    data "Console output; video/sound: PEEK/POKE", 0x0A
VIDEO_HELP_END:
    data "Lines: LIST RUN NEW SAVE LOAD QUIT", 0x0A
    data "BSAVE file,address,length; BLOAD file,address", 0x0A
    data "Binary 0000-BFFF", 0x0A
    data "LET PRINT PRINT AT row,col;items INPUT IF THEN GOTO", 0x0A
    data "GOSUB RETURN FOR NEXT END DEF FNA(X)=expr", 0x0A
    data "A-Z signed: + - * / ()", 0x0A
    data "DIM A(2,3): 1..3 dimensions; 2048 max elements", 0x0A
    data "A$..Z$: 255 ASCII; DIM A$(N), +, LEN; 4K pool", 0x0A
    data "DATA; READ A,A(I),A$; RESTORE [line]", 0x0A
    data "PEEK(address), POKE address,byte: memory and devices", 0x0A
    data "RND(n): 0..n-1; RANDOMIZE [seed] (requires --random-device)", 0x0A, 0x00

; Diagnostic handlers share stack reset, file abort, line context and prompt recovery.
INVALID_LINE_NUMBER:
    ld cd, INVALID_LINE_NUMBER_TEXT
    jmp REPORT_ERROR
INVALID_LINE_NUMBER_TEXT:
    data "? INVALID LINE NUMBER", 0x00
PROGRAM_FULL:
    ld cd, PROGRAM_FULL_TEXT
    jmp REPORT_ERROR
PROGRAM_FULL_TEXT:
    data "? PROGRAM FULL", 0x00
UNDEFINED_LINE:
    ld cd, UNDEFINED_LINE_TEXT
    jmp REPORT_ERROR
UNDEFINED_LINE_TEXT:
    data "? UNDEFINED LINE", 0x00
EXPECTED_EQUALS:
    ld cd, EXPECTED_EQUALS_TEXT
    jmp REPORT_ERROR
EXPECTED_EQUALS_TEXT:
    data "? EXPECTED =", 0x00
UNTERMINATED_STRING:
    ld cd, UNTERMINATED_STRING_TEXT
    jmp REPORT_ERROR
UNTERMINATED_STRING_TEXT:
    data "? UNTERMINATED STRING", 0x00
EXPRESSION_TOO_DEEP:
    ld cd, EXPRESSION_TOO_DEEP_TEXT
    jmp REPORT_ERROR
EXPRESSION_TOO_DEEP_TEXT:
    data "? EXPRESSION TOO DEEP", 0x00
EXPECTED_RPAREN:
    ld cd, EXPECTED_RPAREN_TEXT
    jmp REPORT_ERROR
EXPECTED_RPAREN_TEXT:
    data "? EXPECTED )", 0x00
EXPECTED_VARIABLE:
    ld cd, EXPECTED_VARIABLE_TEXT
    jmp REPORT_ERROR
EXPECTED_VARIABLE_TEXT:
    data "? EXPECTED VARIABLE A-Z", 0x00
PROGRAM_ONLY:
    ld cd, PROGRAM_ONLY_TEXT
    jmp REPORT_ERROR
PROGRAM_ONLY_TEXT:
    data "? REQUIRES RUN", 0x00
EXPECTED_COMPARISON:
    ld cd, EXPECTED_COMPARISON_TEXT
    jmp REPORT_ERROR
EXPECTED_COMPARISON_TEXT:
    data "? EXPECTED COMPARISON", 0x00
EXPECTED_THEN:
    ld cd, EXPECTED_THEN_TEXT
    jmp REPORT_ERROR
EXPECTED_THEN_TEXT:
    data "? EXPECTED THEN", 0x00
STANDALONE_LOOP:
    ld cd, STANDALONE_LOOP_TEXT
    jmp REPORT_ERROR
STANDALONE_LOOP_TEXT:
    data "? FOR/NEXT MUST STAND ALONE", 0x00
GOSUB_STACK_FULL:
    ld cd, GOSUB_STACK_FULL_TEXT
    jmp REPORT_ERROR
GOSUB_STACK_FULL_TEXT:
    data "? GOSUB STACK FULL", 0x00
RETURN_WITHOUT_GOSUB:
    ld cd, RETURN_WITHOUT_GOSUB_TEXT
    jmp REPORT_ERROR
RETURN_WITHOUT_GOSUB_TEXT:
    data "? RETURN WITHOUT GOSUB", 0x00
EXPECTED_TO:
    ld cd, EXPECTED_TO_TEXT
    jmp REPORT_ERROR
EXPECTED_TO_TEXT:
    data "? EXPECTED TO", 0x00
ZERO_STEP:
    ld cd, ZERO_STEP_TEXT
    jmp REPORT_ERROR
ZERO_STEP_TEXT:
    data "? ZERO STEP", 0x00
LOOP_VARIABLE_ACTIVE:
    ld cd, LOOP_VARIABLE_ACTIVE_TEXT
    jmp REPORT_ERROR
LOOP_VARIABLE_ACTIVE_TEXT:
    data "? FOR VARIABLE ALREADY ACTIVE", 0x00
FOR_WITHOUT_NEXT:
    ld cd, FOR_WITHOUT_NEXT_TEXT
    jmp REPORT_ERROR
FOR_WITHOUT_NEXT_TEXT:
    data "? FOR WITHOUT NEXT", 0x00
NEXT_MISMATCH:
    ld cd, NEXT_MISMATCH_TEXT
    jmp REPORT_ERROR
NEXT_MISMATCH_TEXT:
    data "? NEXT MISMATCH", 0x00
FOR_STACK_FULL:
    ld cd, FOR_STACK_FULL_TEXT
    jmp REPORT_ERROR
FOR_STACK_FULL_TEXT:
    data "? FOR STACK FULL", 0x00
NEXT_WITHOUT_FOR:
    ld cd, NEXT_WITHOUT_FOR_TEXT
    jmp REPORT_ERROR
NEXT_WITHOUT_FOR_TEXT:
    data "? NEXT WITHOUT FOR", 0x00
DIRECT_ONLY:
    ld cd, DIRECT_ONLY_TEXT
    jmp REPORT_ERROR
DIRECT_ONLY_TEXT:
    data "? DIRECT MODE ONLY", 0x00
EXPECTED_FILENAME:
    ld cd, EXPECTED_FILENAME_TEXT
    jmp REPORT_ERROR
EXPECTED_FILENAME_TEXT:
    data "? EXPECTED QUOTED FILENAME", 0x00

; A rejected input line must not leave its tail to become another command.
BAD_INPUT_CHARACTER:
    ld cd, BAD_INPUT_CHARACTER_TEXT
    jmp DISCARD_INPUT_LINE
INPUT_TOO_LONG:
    ld cd, INPUT_TOO_LONG_TEXT
DISCARD_INPUT_LINE:
    ld a, 0x00
    ld 0x00EE, a
    push cd
    ld cd, 0x00A6
    ld a, (cd)
    cmp a, 0x00
    bne DISCARD_FILE_BYTE
DISCARD_CONSOLE_WAIT:
    ld cd, 0xFF00
    ld a, (cd)
    ld b, a
    and a, 0x01
    cmp a, 0x00
    bne DISCARD_CONSOLE_BYTE
    and b, 0x02
    cmp b, 0x00
    beq DISCARD_CONSOLE_WAIT
    jmp DISCARD_DONE
DISCARD_CONSOLE_BYTE:
    ld cd, 0xFF01
    ld a, (cd)
    jmp DISCARD_CHECK_BYTE
DISCARD_FILE_BYTE:
    call FILE_CHECK
    ld cd, 0xFF11
    ld a, (cd)
    and a, 0x02
    cmp a, 0x00
    bne DISCARD_DONE
    ld cd, 0xFF13
    ld a, (cd)
DISCARD_CHECK_BYTE:
    cmp a, 0x0A
    beq DISCARD_DONE
    pop cd
    jmp DISCARD_INPUT_LINE
DISCARD_DONE:
    pop cd
    jmp REPORT_ERROR
BAD_INPUT_CHARACTER_TEXT:
    data "? INVALID CHARACTER", 0x00
INPUT_TOO_LONG_TEXT:
    data "? LINE TOO LONG", 0x00

; DEF FNA(X)=expression. The 26 body pointers occupy 0374..03A7.
DEF_STATEMENT:
    call REQUIRE_RUN
    ret
FN_BUILD_DIRECTORY:
    call FN_CLEAR_DIRECTORY
    ld cd, 0x1000
FN_SCAN_NEXT:
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, 0x0000
    beq FN_SCAN_DONE
    push ef
    ld f, (cd+0x02)
    ld e, (cd+0x03)
    ld 0x0080, f
    ld 0x0081, e
    ld gh, cd
    clc
    add gh, 0x0004
    ld cd, KW_DEF
    call MATCH
    bne FN_SCAN_SKIP
    call FN_PARSE_DEFINITION
FN_SCAN_SKIP:
    pop cd
    jmp FN_SCAN_NEXT
FN_SCAN_DONE:
    ret
FN_CLEAR_DIRECTORY:
    ld cd, 0x0374
    ld a, 0x00
FN_CLEAR_NEXT:
    ld (cd), a
    inc cd
    cmp cd, 0x03A8
    bne FN_CLEAR_NEXT
    ret
FN_PARSE_DEFINITION:
    call SPACE
    cmp a, 0x46
    beq FN_DEF_F
    jmp FN_BAD_DEFINITION
FN_DEF_F:
    inc gh
    call PEEK
    cmp a, 0x4E
    beq FN_DEF_N
    jmp FN_BAD_DEFINITION
FN_DEF_N:
    inc gh
    call PEEK
    cmp a, 0x41
    bcs FN_DEF_NAME_LOWER_OK
    jmp FN_BAD_DEFINITION
FN_DEF_NAME_LOWER_OK:
    cmp a, 0x5B
    bcc FN_DEF_NAME_OK
    jmp FN_BAD_DEFINITION
FN_DEF_NAME_OK:
    sec
    sub a, 0x41
    ld b, a
    ld a, 0x00
    shl ab, 0x01
    clc
    add ab, 0x0374
    ld 0x03CC, b
    ld 0x03CD, a
    inc gh
    call SPACE
    cmp a, 0x28
    beq FN_DEF_OPEN
    jmp FN_BAD_DEFINITION
FN_DEF_OPEN:
    inc gh
    call VARIABLE
    call SPACE
    cmp a, 0x29
    beq FN_DEF_CLOSE
    jmp FN_BAD_DEFINITION
FN_DEF_CLOSE:
    inc gh
    call SPACE
    cmp a, 0x3D
    beq FN_DEF_EQUAL
    jmp FN_BAD_DEFINITION
FN_DEF_EQUAL:
    inc gh
    call SPACE
    cmp a, 0x00
    bne FN_DEF_BODY
    jmp FN_BAD_DEFINITION
FN_DEF_BODY:
    ld cd, 0x03CC
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, ef
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, 0x0000
    beq FN_DEF_NEW
    jmp FN_DUPLICATE
FN_DEF_NEW:
    ld cd, 0x03CC
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, ef
    ld (cd), h
    ld (cd+0x01), g
    ret
FN_CALL:
    ld cd, gh
    clc
    add cd, 0x0002
    ld a, (cd)
    sec
    sub a, 0x41
    ld b, a
    ld a, 0x00
    shl ab, 0x01
    clc
    add ab, 0x0374
    ld cd, ab
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, 0x0000
    bne FN_CALL_DEFINED
    jmp FN_UNDEFINED
FN_CALL_DEFINED:
    push ef
    clc
    add gh, 0x0003
    call SPACE
    cmp a, 0x28
    beq FN_CALL_OPEN
    jmp EXPECTED_LPAREN
FN_CALL_OPEN:
    inc gh
    call IS_STRING
    beq FN_CALL_NUMERIC
    jmp TYPE_MISMATCH
FN_CALL_NUMERIC:
    call EXPR
    push ab
    call SPACE
    cmp a, 0x29
    beq FN_CALL_CLOSE
    jmp EXPECTED_RPAREN
FN_CALL_CLOSE:
    inc gh
    pop ab
    pop ef
    ld 0x03C6, b
    ld 0x03C7, a
    ld 0x03C8, f
    ld 0x03C9, e
    push gh
    ld cd, 0x03C2
    ld b, (cd)
    ld a, (cd+0x01)
    push ab
    ld b, (cd+0x02)
    ld a, (cd+0x03)
    push ab
    ld gh, ef
    sec
    sub gh, 0x0003
    call VARIABLE
    ld 0x03C2, d
    ld 0x03C3, c
    ld cd, 0x03C6
    ld b, (cd)
    ld a, (cd+0x01)
    ld 0x03C4, b
    ld 0x03C5, a
    ld cd, 0x03C8
    ld h, (cd)
    ld g, (cd+0x01)
    call EXPR
    call EOL
    ld 0x03CA, b
    ld 0x03CB, a
    pop ab
    ld 0x03C4, b
    ld 0x03C5, a
    pop ab
    ld 0x03C2, b
    ld 0x03C3, a
    pop gh
    ld cd, 0x03CA
    ld b, (cd)
    ld a, (cd+0x01)
    ret
FN_BAD_DEFINITION:
    ld cd, FN_BAD_DEFINITION_TEXT
    jmp REPORT_ERROR
FN_BAD_DEFINITION_TEXT:
    data "? BAD DEFINITION", 0x00
FN_DUPLICATE:
    ld cd, FN_DUPLICATE_TEXT
    jmp REPORT_ERROR
FN_DUPLICATE_TEXT:
    data "? DUPLICATE FUNCTION", 0x00
FN_UNDEFINED:
    ld cd, FN_UNDEFINED_TEXT
    jmp REPORT_ERROR
FN_UNDEFINED_TEXT:
    data "? UNDEFINED FUNCTION", 0x00

; Arrays: 26 descriptors at 0800..0867 (base word, inclusive upper word).
; Elements occupy 9000..9FFF, little endian. 00B0 holds the next free byte.
CLEAR_ARRAYS:
    ld cd, 0x0800
    ld a, 0x00
CLEAR_ARRAY_DESCRIPTOR:
    ld (cd), a
    inc cd
    cmp cd, 0x08D8
    bne CLEAR_ARRAY_DESCRIPTOR
    ld ab, 0x9000
    ld 0x00B0, b
    ld 0x00B1, a
    jmp CLEAR_STRINGS
; Convert scalar variable address CD to the corresponding array descriptor.
ARRAY_DESCRIPTOR:
    push ab
    ld ab, cd
    sec
    sub ab, 0x0300
    shl ab, 0x01
    or ab, 0x0800
    ld cd, ab
    pop ab
    ret
; Parse a scalar or array element location into CD. GH advances past subscript.
LOCATION:
    call VARIABLE
    push ef
    push cd
    call SPACE
    pop cd
    cmp a, 0x24
    bne LOCATION_INTEGER
    jmp TYPE_MISMATCH
LOCATION_INTEGER:
    cmp a, 0x28
    beq LOCATION_ARRAY
    pop ef
    ret
LOCATION_ARRAY:
    call ARRAY_DESCRIPTOR
LOCATION_SUBSCRIPT:
    ; Save the working words: a subscript expression may itself index an array.
    ld ef, cd
    ld cd, 0x03B8
    ld b, (cd)
    ld a, (cd+0x01)
    push ab
    ld b, (cd+0x02)
    ld a, (cd+0x03)
    push ab
    ld b, (cd+0x04)
    ld a, (cd+0x05)
    push ab
    ld b, (cd+0x06)
    ld a, (cd+0x07)
    push ab
    ld cd, ef
    ld 0x03B8, d
    ld 0x03B9, c
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, 0x0000
    bne LOCATION_HAVE_BASE
    jmp ARRAY_NOT_DIMENSIONED
LOCATION_HAVE_BASE:
    ld a, (cd+0x03)
    and a, 0x18
    cmp a, 0x00
    beq LOCATION_RANK_ONE
    cmp a, 0x08
    beq LOCATION_RANK_TWO
    ld ab, 0x0003
    jmp LOCATION_RANK_READY
LOCATION_RANK_TWO:
    ld ab, 0x0002
    jmp LOCATION_RANK_READY
LOCATION_RANK_ONE:
    ld ab, 0x0001
LOCATION_RANK_READY:
    ld 0x03BE, b
    ld 0x03BF, a
    cmp ab, 0x0001
    beq LOCATION_ONE_POINTER
    shl ab, 0x01
    sec
    sub ef, ab
    jmp LOCATION_POINTER_READY
LOCATION_ONE_POINTER:
    ld ef, cd
    clc
    add ef, 0x0002
LOCATION_POINTER_READY:
    ld 0x03BA, f
    ld 0x03BB, e
    ld ab, 0x0000
    ld 0x03BC, b
    ld 0x03BD, a
    inc gh
LOCATION_INDEX:
    call EXPR
    push ab
    call SPACE
    pop ab
    cmp a, 0x80
    bcc LOCATION_INDEX_POSITIVE
    jmp BAD_SUBSCRIPT
LOCATION_INDEX_POSITIVE:
    push ab
    ld cd, 0x03B8
    ld b, (cd)
    ld a, (cd+0x01)
    ld cd, ab
    ld a, (cd+0x03)
    and a, 0x18
    cmp a, 0x00
    bne LOCATION_MULTI_LENGTH
    ld cd, 0x03BA
    ld b, (cd)
    ld a, (cd+0x01)
    ld cd, ab
    ld f, (cd)
    ld e, (cd+0x01)
    and e, 0x07
    inc ef
    jmp LOCATION_LENGTH_READY
LOCATION_MULTI_LENGTH:
    ld cd, 0x03BA
    ld b, (cd)
    ld a, (cd+0x01)
    ld cd, ab
    ld f, (cd)
    ld e, (cd+0x01)
LOCATION_LENGTH_READY:
    pop ab
    cmp ab, ef
    bcc LOCATION_INDEX_FITS
    jmp BAD_SUBSCRIPT
LOCATION_INDEX_FITS:
    push ab
    ld cd, 0x03BC
    ld b, (cd)
    ld a, (cd+0x01)
    call MUL_SIGNED
    pop ef
    call ADD_SIGNED
    ld 0x03BC, b
    ld 0x03BD, a
    ld cd, 0x03BA
    ld b, (cd)
    ld a, (cd+0x01)
    clc
    add ab, 0x0002
    ld (cd), b
    ld (cd+0x01), a
    ld cd, 0x03BE
    ld b, (cd)
    dec b
    ld (cd), b
    call SPACE
    cmp b, 0x00
    beq LOCATION_LAST_INDEX
    cmp a, 0x2C
    beq LOCATION_MORE_INDEX
    jmp BAD_SUBSCRIPT
LOCATION_MORE_INDEX:
    inc gh
    jmp LOCATION_INDEX
LOCATION_LAST_INDEX:
    cmp a, 0x29
    beq LOCATION_END_INDEX
    jmp BAD_SUBSCRIPT
LOCATION_END_INDEX:
    inc gh
    ld cd, 0x03BC
    ld b, (cd)
    ld a, (cd+0x01)
    shl ab, 0x01
    ld cd, 0x03B8
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, ef
    ld f, (cd)
    ld e, (cd+0x01)
    clc
    add ef, ab
    ld 0x03C0, f
    ld 0x03C1, e
    ld cd, 0x03B8
    pop ab
    ld (cd+0x06), b
    ld (cd+0x07), a
    pop ab
    ld (cd+0x04), b
    ld (cd+0x05), a
    pop ab
    ld (cd+0x02), b
    ld (cd+0x03), a
    pop ab
    ld (cd), b
    ld (cd+0x01), a
    ld cd, 0x03C0
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, ef
    pop ef
    ret
DIM:
    call VARIABLE
    call PEEK
    cmp a, 0x24
    bne DIM_INTEGER_ARRAY
    inc gh
    call ARRAY_DESCRIPTOR
    clc
    add cd, 0x0070
    jmp DIM_DESCRIPTOR_READY
DIM_INTEGER_ARRAY:
    call ARRAY_DESCRIPTOR
DIM_DESCRIPTOR_READY:
    ld 0x03B0, d
    ld 0x03B1, c
    call SPACE
    cmp a, 0x28
    beq DIM_OPEN
    jmp EXPECTED_LPAREN
DIM_OPEN:
    inc gh
    ld a, 0x00
    ld 0x03AE, a
DIM_NEXT_BOUND:
    call EXPR
    cmp a, 0x80
    bcc DIM_NONNEGATIVE
    jmp BAD_SUBSCRIPT
DIM_NONNEGATIVE:
    cmp ab, 0x0800
    bcc DIM_BOUND_FITS
    jmp ARRAY_MEMORY_FULL
DIM_BOUND_FITS:
    inc ab
    push ab
    ld cd, 0x03AE
    ld b, (cd)
    inc b
    ld (cd), b
    ld a, 0x00
    shl ab, 0x01
    clc
    add ab, 0x03A8
    sec
    sub ab, 0x0002
    ld cd, ab
    pop ab
    ld (cd), b
    ld (cd+0x01), a
    call SPACE
    cmp a, 0x2C
    bne DIM_END_BOUNDS
    ld cd, 0x03AE
    ld a, (cd)
    cmp a, 0x03
    bcc DIM_COMMA_OK
    jmp BAD_SUBSCRIPT
DIM_COMMA_OK:
    inc gh
    jmp DIM_NEXT_BOUND
DIM_END_BOUNDS:
    cmp a, 0x29
    beq DIM_CLOSE
    jmp EXPECTED_RPAREN
DIM_CLOSE:
    inc gh
    call EOL
    ld cd, 0x03B0
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, ef
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, 0x0000
    beq DIM_UNALLOCATED
    jmp ARRAY_ALREADY_DIMENSIONED
DIM_UNALLOCATED:
    ld cd, 0x03AE
    ld c, (cd)
    ld ab, 0x0001
    ld gh, 0x03A8
DIM_PRODUCT:
    ld f, (gh)
    ld e, (gh+0x01)
    call MUL_SIGNED
    cmp ab, 0x0801
    bcc DIM_PRODUCT_FITS
    jmp ARRAY_MEMORY_FULL
DIM_PRODUCT_FITS:
    inc gh
    inc gh
    dec c
    bne DIM_PRODUCT
    ld 0x03B2, b
    ld 0x03B3, a
    ld cd, 0x00B0
    ld h, (cd)
    ld g, (cd+0x01)
    ld cd, 0x03AE
    ld b, (cd)
    ld a, 0x00
    cmp ab, 0x0001
    beq DIM_NO_METADATA
    shl ab, 0x01
    jmp DIM_METADATA_SIZE
DIM_NO_METADATA:
    ld ab, 0x0000
DIM_METADATA_SIZE:
    ld ef, gh
    clc
    add ef, ab
    ld 0x03B4, f
    ld 0x03B5, e
    ld cd, 0x03B2
    ld b, (cd)
    ld a, (cd+0x01)
    shl ab, 0x01
    clc
    add ef, ab
    cmp ef, 0xA000
    bcc DIM_FITS
    beq DIM_FITS
    jmp ARRAY_MEMORY_FULL
DIM_FITS:
    ld 0x03B6, f
    ld 0x03B7, e
    ld cd, 0x03B0
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, ef
    ld ef, 0x03B4
    ld b, (ef)
    ld a, (ef+0x01)
    ld (cd), b
    ld (cd+0x01), a
    ld ef, 0x03B2
    ld b, (ef)
    ld a, (ef+0x01)
    dec ab
    ld ef, 0x03AE
    ld e, (ef)
    cmp e, 0x02
    bne DIM_NOT_TWO
    or a, 0x08
DIM_NOT_TWO:
    cmp e, 0x03
    bne DIM_RANK_READY
    or a, 0x10
DIM_RANK_READY:
    ld (cd+0x02), b
    ld (cd+0x03), a
    ld cd, 0x00B0
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, ef
    ld ef, 0x03A8
    ld b, 0x03
    ld a, 0xAE
    ; Copy dimension lengths only for rank two or three.
    ld gh, 0x03AE
    ld b, (gh)
    cmp b, 0x01
    beq DIM_CLEAR_ELEMENTS
DIM_COPY_DIMENSION:
    ld a, (ef)
    ld (cd), a
    inc ef
    inc cd
    ld a, (ef)
    ld (cd), a
    inc ef
    inc cd
    dec b
    bne DIM_COPY_DIMENSION
DIM_CLEAR_ELEMENTS:
    ld gh, 0x03B4
    ld f, (gh)
    ld e, (gh+0x01)
    ld gh, ef
    ld cd, 0x03B6
    ld f, (cd)
    ld e, (cd+0x01)
    ld 0x00B0, f
    ld 0x00B1, e
    ld a, 0x00
DIM_CLEAR:
    ld (gh), a
    inc gh
    cmp gh, ef
    bne DIM_CLEAR
    ret
ARRAY_NOT_DIMENSIONED:
    ld cd, ARRAY_NOT_DIMENSIONED_TEXT
    jmp REPORT_ERROR
ARRAY_NOT_DIMENSIONED_TEXT:
    data "? ARRAY NOT DIMENSIONED", 0x00
ARRAY_ALREADY_DIMENSIONED:
    ld cd, ARRAY_ALREADY_DIMENSIONED_TEXT
    jmp REPORT_ERROR
ARRAY_ALREADY_DIMENSIONED_TEXT:
    data "? ARRAY ALREADY DIMENSIONED", 0x00
BAD_SUBSCRIPT:
    ld cd, BAD_SUBSCRIPT_TEXT
    jmp REPORT_ERROR
BAD_SUBSCRIPT_TEXT:
    data "? SUBSCRIPT OUT OF RANGE", 0x00
ARRAY_MEMORY_FULL:
    ld cd, ARRAY_MEMORY_FULL_TEXT
    jmp REPORT_ERROR
ARRAY_MEMORY_FULL_TEXT:
    data "? ARRAY MEMORY FULL", 0x00
EXPECTED_LPAREN:
    ld cd, EXPECTED_LPAREN_TEXT
    jmp REPORT_ERROR
EXPECTED_LPAREN_TEXT:
    data "? EXPECTED (", 0x00

; Look through leading parentheses, without consuming source. Z means numeric.
; Preserves CD/EF/GH; AB is scratch.
IS_STRING:
    push gh
IS_STRING_START:
    call SPACE
    cmp a, 0x28
    bne IS_STRING_ATOM
    inc gh
    jmp IS_STRING_START
IS_STRING_ATOM:
    cmp a, 0xB1
    beq IS_STRING_YES
    cmp a, 0xA0
    bcc IS_STRING_ASCII
    cmp a, 0xA4
    bcc IS_STRING_YES
IS_STRING_ASCII:
    cmp a, 0x22
    beq IS_STRING_YES
    cmp a, 0x41
    bcc IS_STRING_NO
    cmp a, 0x5B
    bcs IS_STRING_NO
IS_STRING_WORD:
    inc gh
    call PEEK
    cmp a, 0x41
    bcc IS_STRING_SUFFIX
    cmp a, 0x5B
    bcc IS_STRING_WORD
IS_STRING_SUFFIX:
    cmp a, 0x24
    beq IS_STRING_YES
IS_STRING_NO:
    ld a, 0x00
    jmp IS_STRING_DONE
IS_STRING_YES:
    ld a, 0x01
IS_STRING_DONE:
    pop gh
    cmp a, 0x00
    ret

; Scalars A$..Z$: one-based pool offsets at 0340..0373.
; Length-prefixed records live in 8000..8FFF. Free records are 0,length.
; 00B2 bump, 00B6 temporary top, 00B8 statement mark, 00C0 dirty.
CLEAR_STRINGS:
    ld cd, 0x0340
    ld a, 0x00
CLEAR_STRING_OFFSET:
    ld (cd), a
    inc cd
    cmp cd, 0x0374
    bne CLEAR_STRING_OFFSET
    ld 0x00C0, a
    ld ab, 0x8000
    ld 0x00B2, b
    ld 0x00B3, a
    ld ab, 0x9000
    ld 0x00B6, b
    ld 0x00B7, a
    ld 0x00B8, b
    ld 0x00B9, a
    jmp RESET_DATA
POOL_BEGIN:
    ld cd, 0x00B6
    ld b, (cd)
    ld a, (cd+0x01)
    cmp ab, 0x9000
    bne POOL_MARK
    ld cd, 0x00C0
    ld a, (cd)
    cmp a, 0x00
    beq POOL_MARK
    call POOL_COMPACT
POOL_MARK:
    ld cd, 0x00B6
    ld b, (cd)
    ld a, (cd+0x01)
    ld 0x00B8, b
    ld 0x00B9, a
    ret
STACK_RESET:
    ld cd, 0x00B8
    ld b, (cd)
    ld a, (cd+0x01)
    ld 0x00B6, b
    ld 0x00B7, a
    ret
STACK_TOP:
    ld cd, 0x00B6
    ld b, (cd)
    ld a, (cd+0x01)
    ld cd, ab
    ret
STACK_POP:
    push ab
    push cd
    call STACK_TOP
    ld b, (cd)
    ld a, 0x00
    inc ab
    clc
    add cd, ab
    ld 0x00B6, d
    ld 0x00B7, c
    pop cd
    pop ab
    ret
; Copy the length-prefixed record at CD onto the temporary stack.
STACK_PUSH:
    push gh
    ld b, (cd)
    ld a, 0x00
    inc ab
    ld gh, ab
    ld ef, 0x00B6
    ld b, (ef)
    ld a, (ef+0x01)
    sec
    sub ab, gh
    ld ef, ab
    push cd
    ld cd, 0x00B2
    ld b, (cd)
    ld a, (cd+0x01)
    pop cd
    cmp ef, ab
    bcc STACK_SPACE_ERROR
    beq STACK_SPACE_ERROR
    ld 0x00B6, f
    ld 0x00B7, e
STACK_PUSH_COPY:
    ld a, (cd)
    ld (ef), a
    inc cd
    inc ef
    dec gh
    bne STACK_PUSH_COPY
    pop gh
    ret
STACK_SPACE_ERROR:
    jmp STRING_SPACE
; Scratch text is NUL terminated; stable and temporary records are not.
STACK_FROM_SCRATCH:
    ld cd, 0x0E01
    ld b, 0x00
STACK_SCRATCH_LENGTH:
    ld a, (cd)
    cmp a, 0x00
    beq STACK_SCRATCH_READY
    inc b
    inc cd
    jmp STACK_SCRATCH_LENGTH
STACK_SCRATCH_READY:
    ld 0x0E00, b
    ld cd, 0x0E00
    jmp STACK_PUSH
; Copy B bytes from CD to EF. GH is untouched.
COPY_STRING_BYTES:
    cmp b, 0x00
    beq COPY_STRING_DONE
COPY_STRING_BYTE:
    ld a, (cd)
    ld (ef), a
    inc cd
    inc ef
    dec b
    bne COPY_STRING_BYTE
COPY_STRING_DONE:
    ret
STRING_JOIN:
    push gh
    call STACK_TOP
    ld gh, cd
    ld b, (cd)
    ld a, 0x00
    inc ab
    clc
    add cd, ab
    ld b, (cd)
    ld a, 0x00
    ld ef, ab
    ld b, (gh)
    clc
    add ab, ef
    cmp ab, 0x0100
    bcc STRING_JOIN_FITS
    jmp STRING_TOO_LONG
STRING_JOIN_FITS:
    ld 0x0B00, b
    ld b, (cd)
    inc cd
    ld ef, 0x0B01
    call COPY_STRING_BYTES
    ld cd, gh
    ld b, (cd)
    inc cd
    call COPY_STRING_BYTES
    call STACK_POP
    call STACK_POP
    ld cd, 0x0B00
    call STACK_PUSH
    pop gh
    ret
STRING_VARIABLE:
    call VARIABLE
    call PEEK
    cmp a, 0x24
    beq STRING_VARIABLE_SUFFIX
    jmp TYPE_MISMATCH
STRING_VARIABLE_SUFFIX:
    inc gh
    push cd
    call SPACE
    pop cd
    cmp a, 0x28
    beq STRING_ARRAY_LOCATION
    or cd, 0x0040
    ret
STRING_ARRAY_LOCATION:
    call ARRAY_DESCRIPTOR
    clc
    add cd, 0x0070
    push ef
    jmp LOCATION_SUBSCRIPT
STRING_EXPRESSION:
    push cd
    push ef
    call STRING_SEQUENCE
    pop ef
    pop cd
    ret
STRING_SEQUENCE:
    cmp sp, 0xA000
    bcs STRING_DEPTH_OK
    jmp EXPRESSION_TOO_DEEP
STRING_DEPTH_OK:
    call STRING_ATOM
STRING_SEQUENCE_MORE:
    call SPACE
    cmp a, 0x2B
    bne STRING_SEQUENCE_DONE
    inc gh
    call STRING_ATOM
    call STRING_JOIN
    jmp STRING_SEQUENCE_MORE
STRING_SEQUENCE_DONE:
    ret
STRING_ATOM:
    call SPACE
    cmp a, 0xB1
    bne STRING_ATOM_TEXT
    inc gh
    ld cd, gh
    ld b, (gh)
    ld a, 0x00
    inc ab
    clc
    add gh, ab
    jmp STACK_PUSH
STRING_ATOM_TEXT:
    cmp a, 0x22
    beq STRING_LITERAL
STRING_ATOM_NOT_LITERAL:
    cmp a, 0x28
    beq STRING_PAREN
STRING_ATOM_NOT_PAREN:
    call IS_STRING
    bne STRING_NAMED
    jmp TYPE_MISMATCH
STRING_NAMED:
    ld cd, KW_MID
    call MATCH
    bne STRING_NOT_MID
    ld ef, 0x0002
    jmp STRING_SLICE
STRING_NOT_MID:
    ld cd, KW_LEFT
    call MATCH
    bne STRING_NOT_LEFT
    ld ef, 0x0000
    jmp STRING_SLICE
STRING_NOT_LEFT:
    ld cd, KW_RIGHT
    call MATCH
    bne STRING_NOT_RIGHT
    ld ef, 0x0001
    jmp STRING_SLICE
STRING_NOT_RIGHT:
    ld cd, KW_CHR
    call MATCH
    bne STRING_NOT_CHR
    jmp STRING_CHR
STRING_NOT_CHR:
    call STRING_VARIABLE
    ld b, (cd)
    ld a, (cd+0x01)
    cmp ab, 0x0000
    beq STRING_EMPTY
    clc
    add ab, 0x7FFF
    ld cd, ab
    jmp STACK_PUSH
STRING_EMPTY:
    ld cd, 0x0E00
    ld (cd), 0x00
    jmp STACK_PUSH
STRING_PAREN:
    inc gh
    call STRING_SEQUENCE
    call SPACE
    cmp a, 0x29
    beq STRING_PAREN_CLOSE
    jmp EXPECTED_RPAREN
STRING_PAREN_CLOSE:
    inc gh
    ret
STRING_LITERAL:
    inc gh
    ld ef, 0x0E01
STRING_LITERAL_BYTE:
    ld a, (gh)
    inc gh
    cmp a, 0x00
    bne STRING_LITERAL_NOT_END
    jmp UNTERMINATED_STRING
STRING_LITERAL_NOT_END:
    cmp a, 0x22
    beq STRING_LITERAL_END
    call STRING_APPEND
    jmp STRING_LITERAL_BYTE
STRING_LITERAL_END:
    ld (ef), 0x00
    jmp STACK_FROM_SCRATCH
STRING_APPEND:
    cmp a, 0x09
    beq STRING_APPEND_VALID
    cmp a, 0x20
    bcs STRING_APPEND_PRINTABLE
    jmp INVALID_STRING_CHARACTER
STRING_APPEND_PRINTABLE:
    cmp a, 0x7F
    bcc STRING_APPEND_VALID
    jmp INVALID_STRING_CHARACTER
STRING_APPEND_VALID:
    cmp ef, 0x0F00
    bcc STRING_APPEND_FITS
    jmp STRING_TOO_LONG
STRING_APPEND_FITS:
    ld (ef), a
    inc ef
    ret
ASSIGN_STRING:
    call STRING_VARIABLE
    push cd
    call SPACE
    cmp a, 0x3D
    beq ASSIGN_STRING_VALUE
    jmp EXPECTED_EQUALS
ASSIGN_STRING_VALUE:
    inc gh
    call STRING_EXPRESSION
    call EOL
    pop ef
    jmp POOL_STORE
INPUT_STRING:
    call SC_INPUT_BEGIN
    call STRING_VARIABLE
    push cd
    call EOL
    ld a, 0x3F
    call PUTCHAR
    ld a, 0x20
    call PUTCHAR
    call READLINE_STRING
    ld gh, 0x0200
    ld ef, 0x0E01
INPUT_STRING_BYTE:
    ld a, (gh)
    cmp a, 0x00
    beq INPUT_STRING_END
    call STRING_APPEND
    inc gh
    jmp INPUT_STRING_BYTE
INPUT_STRING_END:
    ld (ef), 0x00
    call STACK_FROM_SCRATCH
    pop ef
    call SC_INPUT_END
    jmp POOL_STORE
PRINT_STACK_STRING:
    call STACK_TOP
    ld b, (cd)
    inc cd
    cmp b, 0x00
    beq PRINT_STACK_DONE
PRINT_STACK_BYTE:
    ld a, (cd)
    call PUTCHAR
    inc cd
    dec b
    bne PRINT_STACK_BYTE
PRINT_STACK_DONE:
    jmp STACK_POP
COMPARE_STACK_STRINGS:
    push gh
    call STACK_TOP
    ld ef, cd
    ld b, (cd)
    ld a, 0x00
    inc ab
    clc
    add cd, ab
    ld g, (cd)
    ld h, (ef)
    inc cd
    inc ef
    call COMPARE_STRING_BYTES
    pop gh
    call STACK_POP
    call STACK_POP
    ret
COMPARE_STRING_BYTES:
    cmp g, 0x00
    bne COMPARE_STRING_LEFT
    cmp h, 0x00
    bne COMPARE_STRING_LESS
    jmp COMP_EQUAL
COMPARE_STRING_LEFT:
    cmp h, 0x00
    beq COMPARE_STRING_GREATER
    ld a, (cd)
    ld b, (ef)
    cmp a, b
    bcc COMPARE_STRING_LESS
    bne COMPARE_STRING_GREATER
    inc cd
    inc ef
    dec g
    dec h
    jmp COMPARE_STRING_BYTES
COMPARE_STRING_LESS:
    jmp COMP_LESS
COMPARE_STRING_GREATER:
    jmp COMP_GREATER
; Copy top temporary into a stable allocation before freeing the old record.
; EF identifies a scalar offset or string-array element.
POOL_STORE:
    push ef
    call STACK_TOP
    ld b, (cd)
    cmp b, 0x00
    beq POOL_STORE_EMPTY
    call POOL_ALLOC
    ld ab, cd
    sec
    sub ab, 0x7FFF
    ld 0x00C2, b
    ld 0x00C3, a
    ld ef, cd
    call STACK_TOP
    ld b, (cd)
    ld a, (cd)
    ld (ef), a
    inc cd
    inc ef
    call COPY_STRING_BYTES
    jmp POOL_STORE_COMMIT
POOL_STORE_EMPTY:
    ld a, 0x00
    ld 0x00C2, a
    ld 0x00C3, a
POOL_STORE_COMMIT:
    pop cd
    ld b, (cd)
    ld a, (cd+0x01)
    push ab
    ld ef, 0x00C2
    ld b, (ef)
    ld a, (ef+0x01)
    ld (cd), b
    ld (cd+0x01), a
    pop ab
    cmp ab, 0x0000
    beq POOL_STORE_DONE
    clc
    add ab, 0x7FFF
    ld cd, ab
    ld a, (cd)
    ld (cd), 0x00
    ld (cd+0x01), a
    ld a, 0x01
    ld 0x00C0, a
POOL_STORE_DONE:
    jmp STACK_POP
; B is requested length; CD receives its record. Preserve EF/GH.
POOL_ALLOC:
    push ef
    push gh
    ld 0x00C1, b
    ld a, 0x00
    inc ab
    ld gh, ab
    ld cd, 0x00B2
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, 0x8000
POOL_ALLOC_SCAN:
    cmp cd, ef
    beq POOL_ALLOC_BUMP
    ld a, (cd)
    cmp a, 0x00
    beq POOL_ALLOC_FREE
    ld b, a
    ld a, 0x00
    inc ab
    clc
    add cd, ab
    jmp POOL_ALLOC_SCAN
POOL_ALLOC_FREE:
    ld b, (cd+0x01)
    ld a, 0x00
    inc ab
    cmp ab, gh
    bcc POOL_ALLOC_SKIP
    beq POOL_ALLOC_READY
    sec
    sub ab, gh
    cmp ab, 0x0001
    beq POOL_ALLOC_ONE_BYTE
    ld ef, cd
    clc
    add ef, gh
    dec ab
    ld (ef), 0x00
    ld (ef+0x01), b
    jmp POOL_ALLOC_READY
POOL_ALLOC_ONE_BYTE:
    clc
    add ab, gh
POOL_ALLOC_SKIP:
    clc
    add cd, ab
    jmp POOL_ALLOC_SCAN
POOL_ALLOC_BUMP:
    ld ab, cd
    clc
    add ab, gh
    ld ef, 0x00B6
    ld f, (ef)
    ; Loading F changes EF, so obtain the high byte through a separate pointer.
    push cd
    ld cd, 0x00B7
    ld e, (cd)
    pop cd
    cmp ab, ef
    bcc POOL_ALLOC_BUMP_OK
    jmp STRING_SPACE
POOL_ALLOC_BUMP_OK:
    ld 0x00B2, b
    ld 0x00B3, a
POOL_ALLOC_READY:
    ld ef, 0x00C1
    ld a, (ef)
    ld (cd), a
    pop gh
    pop ef
    ret
; No temporary records may exist while offsets are rewritten.
POOL_COMPACT:
    push ab
    push cd
    push ef
    push gh
    ld cd, 0x00B2
    ld b, (cd)
    ld a, (cd+0x01)
    ld 0x00C8, b
    ld 0x00C9, a
    ld gh, 0x8000
    ld ef, 0x8000
POOL_COMPACT_NEXT:
    ld cd, 0x00C8
    ld b, (cd)
    ld a, (cd+0x01)
    cmp gh, ab
    beq POOL_COMPACT_DONE
    ld a, (gh)
    cmp a, 0x00
    bne POOL_COMPACT_LIVE
    ld b, (gh+0x01)
    ld a, 0x00
    inc ab
    clc
    add gh, ab
    jmp POOL_COMPACT_NEXT
POOL_COMPACT_LIVE:
    ld b, a
    ld a, 0x00
    inc ab
    cmp gh, ef
    beq POOL_COMPACT_UNMOVED
    push ab
    ld ab, gh
    sec
    sub ab, 0x7FFF
    ld 0x00C4, b
    ld 0x00C5, a
    ld ab, ef
    sec
    sub ab, 0x7FFF
    ld 0x00C6, b
    ld 0x00C7, a
    call POOL_REWRITE
    pop cd
POOL_COMPACT_COPY:
    ld a, (gh)
    ld (ef), a
    inc gh
    inc ef
    dec cd
    bne POOL_COMPACT_COPY
    jmp POOL_COMPACT_NEXT
POOL_COMPACT_UNMOVED:
    clc
    add gh, ab
    clc
    add ef, ab
    jmp POOL_COMPACT_NEXT
POOL_COMPACT_DONE:
    ld 0x00B2, f
    ld 0x00B3, e
    ld a, 0x00
    ld 0x00C0, a
    pop gh
    pop ef
    pop cd
    pop ab
    ret
POOL_REWRITE:
    push gh
    push ef
    ld cd, 0x00C4
    ld h, (cd)
    ld g, (cd+0x01)
    ld cd, 0x0340
POOL_REWRITE_SCALAR:
    call POOL_REWRITE_WORD
    inc cd
    inc cd
    cmp cd, 0x0374
    bne POOL_REWRITE_SCALAR
    ld cd, 0x0870
POOL_REWRITE_ARRAY:
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, 0x0000
    beq POOL_REWRITE_NEXT_ARRAY
    push cd
    push ef
    ld b, (cd+0x02)
    ld a, (cd+0x03)
    and a, 0x07
    inc ab
    shl ab, 0x01
    clc
    add ef, ab
    pop cd
POOL_REWRITE_ELEMENT:
    call POOL_REWRITE_WORD
    inc cd
    inc cd
    cmp cd, ef
    bne POOL_REWRITE_ELEMENT
    pop cd
POOL_REWRITE_NEXT_ARRAY:
    clc
    add cd, 0x0004
    cmp cd, 0x08D8
    bne POOL_REWRITE_ARRAY
    pop ef
    pop gh
    ret
POOL_REWRITE_WORD:
    ld b, (cd)
    ld a, (cd+0x01)
    cmp ab, gh
    bne POOL_REWRITE_WORD_DONE
    push ef
    ld ef, 0x00C6
    ld b, (ef)
    ld a, (ef+0x01)
    ld (cd), b
    ld (cd+0x01), a
    pop ef
POOL_REWRITE_WORD_DONE:
    ret
STRING_SPACE:
    ld cd, STRING_SPACE_TEXT
    jmp REPORT_ERROR
STRING_SPACE_TEXT:
    data "? STRING SPACE", 0x00

; LEN(string expression) or LEN(bare array name), returned as a signed integer.
LENGTH:
    call SPACE
    cmp a, 0x28
    beq LENGTH_OPEN
    jmp EXPECTED_LPAREN
LENGTH_OPEN:
    inc gh
    call IS_STRING
    beq LENGTH_ARRAY
    ; A bare dimensioned A$ denotes its array count, otherwise its scalar.
    push gh
    call SPACE
    cmp a, 0x41
    bcc LENGTH_STRING_EXPRESSION
    cmp a, 0x5B
    bcs LENGTH_STRING_EXPRESSION
    inc gh
    call PEEK
    cmp a, 0x24
    bne LENGTH_STRING_EXPRESSION
    dec gh
    call VARIABLE
    call PEEK
    inc gh
    push cd
    call SPACE
    pop cd
    cmp a, 0x29
    bne LENGTH_STRING_EXPRESSION
    call ARRAY_DESCRIPTOR
    clc
    add cd, 0x0070
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, 0x0000
    beq LENGTH_STRING_EXPRESSION
    pop ab
    jmp LENGTH_ARRAY_DIMENSIONED
LENGTH_STRING_EXPRESSION:
    pop gh
    call STRING_EXPRESSION
    call STACK_TOP
    ld b, (cd)
    ld a, 0x00
    call STACK_POP
    jmp LENGTH_CLOSE
LENGTH_ARRAY:
    call SPACE
    cmp a, 0x41
    bcc TYPE_MISMATCH
LENGTH_ARRAY_LETTER:
    cmp a, 0x5B
    bcs TYPE_MISMATCH
LENGTH_ARRAY_NAME:
    call VARIABLE
    push cd
    call SPACE
    cmp a, 0x29
    bne TYPE_MISMATCH
LENGTH_ARRAY_END:
    pop cd
    call ARRAY_DESCRIPTOR
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, 0x0000
    bne LENGTH_ARRAY_DIMENSIONED
    jmp ARRAY_NOT_DIMENSIONED
LENGTH_ARRAY_DIMENSIONED:
    ld b, (cd+0x02)
    ld a, (cd+0x03)
    and a, 0x07
    inc ab
LENGTH_CLOSE:
    push ab
    call SPACE
    cmp a, 0x29
    beq LENGTH_END
    jmp EXPECTED_RPAREN
LENGTH_END:
    inc gh
    pop ab
    ret
TYPE_MISMATCH:
    ld cd, TYPE_MISMATCH_TEXT
    jmp REPORT_ERROR
TYPE_MISMATCH_TEXT:
    data "? TYPE MISMATCH", 0x00
STRING_TOO_LONG:
    ld cd, STRING_TOO_LONG_TEXT
    jmp REPORT_ERROR
STRING_TOO_LONG_TEXT:
    data "? STRING TOO LONG", 0x00
INVALID_STRING_CHARACTER:
    ld cd, INVALID_STRING_CHARACTER_TEXT
    jmp REPORT_ERROR
INVALID_STRING_CHARACTER_TEXT:
    data "? INVALID STRING CHARACTER", 0x00
; DATA cursor: 00B4 last scanned line, 00BC next item address (zero: scan).
; 00BE quoted flag; 00BA candidate next address, committed after assignment.
RESET_DATA:
    ld a, 0x00
    ld 0x00B4, a
    ld 0x00B5, a
    ld 0x00BC, a
    ld 0x00BD, a
    ret
RESTORE_DATA:
    call SPACE
    cmp a, 0x00
    beq RESET_DATA
RESTORE_LINE:
    cmp a, 0xB2
    beq RESTORE_NUMBER
    cmp a, 0x30
    bcs RESTORE_DIGIT
    jmp INVALID_LINE_NUMBER
RESTORE_DIGIT:
    cmp a, 0x3A
    bcc RESTORE_NUMBER
    jmp INVALID_LINE_NUMBER
RESTORE_NUMBER:
    call NUMBER
    cmp ab, 0x0000
    bne RESTORE_NONZERO
    jmp INVALID_LINE_NUMBER
RESTORE_NONZERO:
    push ab
    call EOL
    pop ab
    dec ab
    call FIND_NEXT
    cmp cd, 0x0000
    bne RESTORE_FOUND
    jmp UNDEFINED_LINE
RESTORE_FOUND:
    inc ab
    cmp ab, ef
    beq RESTORE_EXACT
    jmp UNDEFINED_LINE
RESTORE_EXACT:
    dec ab
    ld 0x00B4, b
    ld 0x00B5, a
    ld a, 0x00
    ld 0x00BC, a
    ld 0x00BD, a
    ret

READ_DATA:
    call IS_STRING
    beq READ_INTEGER
    call STRING_VARIABLE
    push cd
    call READ_TARGET_END
    push gh
    call DATA_ITEM
    pop gh
    call STACK_FROM_SCRATCH
    pop ef
    call POOL_STORE
    jmp READ_COMMIT
READ_INTEGER:
    call LOCATION
    push cd
    call READ_TARGET_END
    push gh
    call DATA_ITEM
    call DATA_INTEGER
    pop gh
    pop cd
    ld (cd), b
    ld (cd+0x01), a
READ_COMMIT:
    ld cd, 0x00BA
    ld a, (cd)
    ld 0x00BC, a
    ld a, (cd+0x01)
    ld 0x00BD, a
    call SPACE
    cmp a, 0x2C
    beq READ_ANOTHER
    ret
READ_ANOTHER:
    inc gh
    jmp READ_DATA
READ_TARGET_END:
    call SPACE
    cmp a, 0x2C
    beq READ_TARGET_OK
    cmp a, 0x00
    beq READ_TARGET_OK
    jmp SYNTAX_ERROR
READ_TARGET_OK:
    ret

; Locate the next DATA item, retaining its address until READ succeeds.
DATA_ITEM:
    ld cd, 0x00BC
    ld h, (cd)
    ld g, (cd+0x01)
    cmp gh, 0x0000
    bne DATA_PARSE
DATA_SCAN:
    ld cd, 0x00B4
    ld b, (cd)
    ld a, (cd+0x01)
    call FIND_NEXT
    cmp cd, 0x0000
    bne DATA_SCAN_LINE
    jmp OUT_OF_DATA
DATA_SCAN_LINE:
    ld 0x00B4, f
    ld 0x00B5, e
    ld gh, cd
    clc
    add gh, 0x0004
    call SPACE
    ld cd, KW_DATA
    call MATCH
    bne DATA_SCAN
    ld 0x00BC, h
    ld 0x00BD, g
DATA_PARSE:
    ld a, 0x00
    ld 0x00BE, a
    call SPACE
    cmp a, 0x22
    bne DATA_UNQUOTED
    ld a, 0x01
    ld 0x00BE, a
    ld ef, 0x0E01
    inc gh
DATA_QUOTED_BYTE:
    ld a, (gh)
    inc gh
    cmp a, 0x00
    bne DATA_QUOTED_NOT_END
    jmp UNTERMINATED_STRING
DATA_QUOTED_NOT_END:
    cmp a, 0x22
    beq DATA_QUOTED_END
    call STRING_APPEND
    jmp DATA_QUOTED_BYTE
DATA_QUOTED_END:
    call SPACE
    jmp DATA_SEPARATOR
DATA_UNQUOTED:
    push gh
    ld cd, gh
DATA_UNQUOTED_SCAN:
    ld a, (gh)
    cmp a, 0x00
    beq DATA_UNQUOTED_END
    cmp a, 0x2C
    beq DATA_UNQUOTED_END
    cmp a, 0x22
    bne DATA_NOT_QUOTE
    jmp INVALID_DATA
DATA_NOT_QUOTE:
    cmp a, 0x3A
    bne DATA_NOT_COLON
    jmp INVALID_DATA
DATA_NOT_COLON:
    inc gh
    cmp a, 0x20
    beq DATA_UNQUOTED_SCAN
    cmp a, 0x09
    beq DATA_UNQUOTED_SCAN
    ld cd, gh
    jmp DATA_UNQUOTED_SCAN
DATA_UNQUOTED_END:
    ld ef, gh
    pop gh
    cmp gh, cd
    bne DATA_UNQUOTED_NONEMPTY
    jmp INVALID_DATA
DATA_UNQUOTED_NONEMPTY:
    push ef
    ld ef, 0x0E01
DATA_UNQUOTED_COPY:
    ld a, (gh)
    call STRING_APPEND
    inc gh
    cmp gh, cd
    bne DATA_UNQUOTED_COPY
    pop gh
    ld a, (gh)
DATA_SEPARATOR:
    ld (ef), 0x00
    cmp a, 0x00
    beq DATA_LINE_DONE
    cmp a, 0x2C
    bne INVALID_DATA
DATA_NEXT_ITEM:
    inc gh
    ld 0x00BA, h
    ld 0x00BB, g
    ret
DATA_LINE_DONE:
    ld a, 0x00
    ld 0x00BA, a
    ld 0x00BB, a
    ret

; Strict signed decimal constants only: no variable lookup or expressions.
DATA_INTEGER:
    ld cd, 0x00BE
    ld a, (cd)
    cmp a, 0x00
    beq DATA_INTEGER_UNQUOTED
    jmp TYPE_MISMATCH
DATA_INTEGER_UNQUOTED:
    ld gh, 0x0E01
    ld c, 0x00
    ld a, (gh)
    cmp a, 0x2D
    bne DATA_INTEGER_PLUS
    ld c, 0x01
    inc gh
    jmp DATA_INTEGER_DIGITS
DATA_INTEGER_PLUS:
    cmp a, 0x2B
    bne DATA_INTEGER_DIGITS
    inc gh
DATA_INTEGER_DIGITS:
    ld a, (gh)
    cmp a, 0x30
    bcs DATA_INTEGER_DIGIT
    jmp TYPE_MISMATCH
DATA_INTEGER_DIGIT:
    cmp a, 0x3A
    bcc DATA_INTEGER_NUMBER
    jmp TYPE_MISMATCH
DATA_INTEGER_NUMBER:
    call NUMBER
    push ab
    ld a, (gh)
    cmp a, 0x00
    beq DATA_INTEGER_END
    jmp TYPE_MISMATCH
DATA_INTEGER_END:
    pop ab
    cmp c, 0x00
    bne DATA_INTEGER_NEGATIVE
    cmp ab, 0x8000
    bcc DATA_INTEGER_RETURN
    jmp OVERFLOW
DATA_INTEGER_NEGATIVE:
    cmp ab, 0x8000
    bcc DATA_INTEGER_NEGATE
    beq DATA_INTEGER_NEGATE
    jmp OVERFLOW
DATA_INTEGER_NEGATE:
    jmp NEGATE
DATA_INTEGER_RETURN:
    ret
OUT_OF_DATA:
    ld cd, OUT_OF_DATA_TEXT
    jmp REPORT_ERROR
OUT_OF_DATA_TEXT:
    data "? OUT OF DATA", 0x00
INVALID_DATA:
    ld cd, INVALID_DATA_TEXT
    jmp REPORT_ERROR
INVALID_DATA_TEXT:
    data "? INVALID DATA", 0x00

; Memory addresses accept a bare unsigned literal or the bits of a signed
; expression. Literal lookahead only parses source; bus reads happen once.
MEM_ADDRESS:
    call SPACE
    push gh
    cmp a, 0xB0
    beq MEM_ADDRESS_LITERAL
    cmp a, 0xB2
    beq MEM_ADDRESS_LITERAL
    cmp a, 0x30
    bcc MEM_ADDRESS_EXPR
MEM_ADDRESS_DIGIT:
    cmp a, 0x3A
    bcs MEM_ADDRESS_EXPR
MEM_ADDRESS_LITERAL:
    call NUMBER
    push ab
    call SPACE
    ld c, a
    pop ab
    cmp c, 0x2C
    beq MEM_ADDRESS_DONE
    cmp c, 0x29
    beq MEM_ADDRESS_DONE
    cmp c, 0x00
    beq MEM_ADDRESS_DONE
MEM_ADDRESS_EXPR:
    pop gh
    jmp EXPR
MEM_ADDRESS_DONE:
    pop cd
    ret
PEEK_BYTE:
    call SPACE
    cmp a, 0x28
    beq PEEK_BYTE_OPEN
    jmp EXPECTED_LPAREN
PEEK_BYTE_OPEN:
    inc gh
    call MEM_ADDRESS
    push ab
    call SPACE
    cmp a, 0x29
    beq PEEK_BYTE_READ
    jmp EXPECTED_RPAREN
PEEK_BYTE_READ:
    inc gh
    pop cd
    ld b, (cd)
    ld a, 0x00
    ret
POKE_BYTE:
    call MEM_ADDRESS
    push ab
    call SPACE
    cmp a, 0x2C
    bne EXPECTED_COMMA
POKE_BYTE_VALUE:
    inc gh
    call EXPR
    cmp a, 0x00
    bne BYTE_OUT_OF_RANGE
POKE_BYTE_VALID:
    push ab
    call EOL
    pop ab
    pop cd
    ld (cd), b
    ret
EXPECTED_COMMA:
    ld cd, EXPECTED_COMMA_TEXT
    jmp REPORT_ERROR
EXPECTED_COMMA_TEXT:
    data "? EXPECTED COMMA", 0x00
BYTE_OUT_OF_RANGE:
    ld cd, BYTE_OUT_OF_RANGE_TEXT
    jmp REPORT_ERROR
BYTE_OUT_OF_RANGE_TEXT:
    data "? BYTE OUT OF RANGE", 0x00

; Optional random device: FF20 byte, FF21 status / entropy command,
; FF22 seed low byte, FF23 seed high byte (commits a 16-bit seed).
RANDOM_DEVICE:
    ld cd, 0xFF21
    ld a, (cd)
    and a, 0x01
    bne RANDOM_DEVICE_READY
    ld cd, RANDOM_MISSING_TEXT
    jmp REPORT_ERROR
RANDOM_DEVICE_READY:
    ret
RANDOMIZE:
    call SPACE
    cmp a, 0x00
    beq RANDOMIZE_ENTROPY
    call EXPR
    push ab
    call EOL
    call RANDOM_DEVICE
    pop ab
    ld cd, 0xFF22
    ld (cd), b
    ld (cd+0x01), a
    ret
RANDOMIZE_ENTROPY:
    call RANDOM_DEVICE
    ld a, 0x01
    ld (cd), a
    ld a, (cd)
    and a, 0x80
    beq RANDOM_DEVICE_READY
    ld cd, RANDOM_SEED_ERROR_TEXT
    jmp REPORT_ERROR
RANDOM_NUMBER:
    call SPACE
    cmp a, 0x28
    beq RANDOM_NUMBER_OPEN
    jmp EXPECTED_LPAREN
RANDOM_NUMBER_OPEN:
    inc gh
    call EXPR
    push ab
    call SPACE
    cmp a, 0x29
    beq RANDOM_NUMBER_CLOSE
    jmp EXPECTED_RPAREN
RANDOM_NUMBER_CLOSE:
    inc gh
    pop ab
    cmp ab, 0x0000
    beq RANDOM_BOUND_ERROR
    cmp ab, 0x8000
    bcs RANDOM_BOUND_ERROR
    push ab
    call RANDOM_DEVICE
    pop ef
    ; Reject the incomplete top interval before taking the remainder.
    ld ab, 0x8000
    call RANDOM_REMAINDER
    ld cd, 0x8000
    sec
    sub cd, ab
RANDOM_NUMBER_DRAW:
    push cd
    ld cd, 0xFF20
    ld a, (cd)
    and a, 0x7F
    ld b, (cd)
    pop cd
    cmp ab, cd
    bcs RANDOM_NUMBER_DRAW
    jmp RANDOM_REMAINDER
RANDOM_BOUND_ERROR:
    ld cd, RANDOM_BOUND_TEXT
    jmp REPORT_ERROR
; Unsigned AB modulo positive EF; preserves CD, EF, GH.
RANDOM_REMAINDER:
    push cd
    push ef
    ld cd, ef
RANDOM_REMAINDER_ALIGN:
    cmp ef, ab
    bcs RANDOM_REMAINDER_SUBTRACT
    shl ef, 0x01
    jmp RANDOM_REMAINDER_ALIGN
RANDOM_REMAINDER_SUBTRACT:
    cmp ab, ef
    bcc RANDOM_REMAINDER_SHIFT
    sec
    sub ab, ef
RANDOM_REMAINDER_SHIFT:
    lsr ef, 0x01
    cmp ef, cd
    bcs RANDOM_REMAINDER_SUBTRACT
    pop ef
    pop cd
    ret
RANDOM_MISSING_TEXT:
    data "? RANDOM DEVICE NOT AVAILABLE", 0x00
RANDOM_BOUND_TEXT:
    data "? INVALID RANDOM BOUND", 0x00
RANDOM_SEED_ERROR_TEXT:
    data "? RANDOM SEED FAILED", 0x00

; String built-ins keep arguments on the temporary stack across recursive calls.
FUNCTION_OPEN:
    call SPACE
    cmp a, 0x28
    beq FUNCTION_OPEN_OK
    jmp EXPECTED_LPAREN
FUNCTION_OPEN_OK:
    inc gh
    ret
FUNCTION_COMMA:
    call SPACE
    cmp a, 0x2C
    beq FUNCTION_COMMA_OK
    jmp EXPECTED_COMMA
FUNCTION_COMMA_OK:
    inc gh
    ret
FUNCTION_NONNEGATIVE:
    cmp ab, 0x8000
    bcc FUNCTION_ARGUMENT_OK
    jmp BAD_FUNCTION_ARGUMENT
FUNCTION_POSITIVE:
    cmp ab, 0x0000
    bne FUNCTION_NONNEGATIVE
    jmp BAD_FUNCTION_ARGUMENT
FUNCTION_ARGUMENT_OK:
    ret
BAD_FUNCTION_ARGUMENT:
    ld cd, BAD_FUNCTION_ARGUMENT_TEXT
    jmp REPORT_ERROR
BAD_FUNCTION_ARGUMENT_TEXT:
    data "? INVALID FUNCTION ARGUMENT", 0x00

; EF selects LEFT (0), RIGHT (1), MID (2).
STRING_SLICE:
    call FUNCTION_OPEN
    call STRING_EXPRESSION
    call FUNCTION_COMMA
    call EXPR
    call FUNCTION_NONNEGATIVE
    cmp ef, 0x0002
    beq STRING_MID_ARGUMENTS
    ; Count in EF, zero-based start in AB.
    ld cd, ef
    ld ef, ab
    ld ab, 0x0000
    cmp cd, 0x0000
    beq STRING_SLICE_ARGUMENTS_DONE
    call STACK_TOP
    ld b, (cd)
    ld a, 0x00
    cmp ab, ef
    bcs STRING_RIGHT_START
    ld ab, 0x0000
    jmp STRING_SLICE_ARGUMENTS_DONE
STRING_RIGHT_START:
    sec
    sub ab, ef
    jmp STRING_SLICE_ARGUMENTS_DONE
STRING_MID_ARGUMENTS:
    call FUNCTION_POSITIVE
    dec ab
    push ab
    call SPACE
    cmp a, 0x2C
    beq STRING_MID_COUNT
    ld ab, 0x00FF
    jmp STRING_MID_COUNT_DONE
STRING_MID_COUNT:
    inc gh
    call EXPR
    call FUNCTION_NONNEGATIVE
STRING_MID_COUNT_DONE:
    ld ef, ab
    pop ab
STRING_SLICE_ARGUMENTS_DONE:
    call LENGTH_CLOSE
    push ab
    call STACK_TOP
    ld b, (cd)
    ld a, 0x00
    pop cd
    ; Clamp start to length, then count to remaining length.
    cmp cd, ab
    bcc STRING_SLICE_START_OK
    ld cd, ab
STRING_SLICE_START_OK:
    sec
    sub ab, cd
    cmp ef, ab
    bcc STRING_SLICE_COUNT_OK
    ld ef, ab
STRING_SLICE_COUNT_OK:
    push cd
    push ef
    call STACK_TOP
    inc cd
    pop ef
    pop ab
    clc
    add cd, ab
    ld b, f
    ld 0x0E00, b
    ld ef, 0x0E01
    call COPY_STRING_BYTES
    call STACK_POP
    ld cd, 0x0E00
    jmp STACK_PUSH
STRING_CHR:
    call FUNCTION_OPEN
    call EXPR
    call LENGTH_CLOSE
    cmp ab, 0x0009
    beq STRING_CHR_VALID
    cmp ab, 0x0020
    bcc STRING_CHR_INVALID
    cmp ab, 0x007F
    bcs STRING_CHR_INVALID
STRING_CHR_VALID:
    ld 0x0E01, b
    ld a, 0x01
    ld 0x0E00, a
    ld cd, 0x0E00
    jmp STACK_PUSH
STRING_CHR_INVALID:
    jmp INVALID_STRING_CHARACTER
STRING_ASC:
    call FUNCTION_OPEN
    call STRING_EXPRESSION
    call STACK_TOP
    ld a, (cd)
    cmp a, 0x00
    bne STRING_ASC_NONEMPTY
    jmp BAD_FUNCTION_ARGUMENT
STRING_ASC_NONEMPTY:
    ld b, (cd+0x01)
    ld a, 0x00
    call STACK_POP
    jmp LENGTH_CLOSE
STRING_VAL:
    call FUNCTION_OPEN
    call STRING_EXPRESSION
    call LENGTH_CLOSE
    ; Scratch has a terminating NUL so NUMBER cannot read past the string.
    call STACK_TOP
    ld b, (cd)
    inc cd
    ld ef, 0x0E01
    call COPY_STRING_BYTES
    ld (ef), 0x00
    call STACK_POP
    push gh
    ld gh, 0x0E01
    call SPACE
    ld c, 0x00
    cmp a, 0x2D
    bne STRING_VAL_PLUS
    ld c, 0x01
    inc gh
    jmp STRING_VAL_NUMBER
STRING_VAL_PLUS:
    cmp a, 0x2B
    bne STRING_VAL_NUMBER
    inc gh
STRING_VAL_NUMBER:
    call NUMBER
    pop gh
    cmp c, 0x00
    bne STRING_VAL_NEGATIVE
    cmp ab, 0x8000
    bcc STRING_VAL_DONE
    jmp OVERFLOW
STRING_VAL_NEGATIVE:
    cmp ab, 0x8000
    bcc STRING_VAL_NEGATE
    beq STRING_VAL_NEGATE
    jmp OVERFLOW
STRING_VAL_NEGATE:
    jmp NEGATE
STRING_VAL_DONE:
    ret
STRING_INSTR:
    call FUNCTION_OPEN
    call IS_STRING
    bne STRING_INSTR_DEFAULT
    call EXPR
    call FUNCTION_POSITIVE
    push ab
    call FUNCTION_COMMA
    jmp STRING_INSTR_ARGUMENTS
STRING_INSTR_DEFAULT:
    ld ab, 0x0001
    push ab
STRING_INSTR_ARGUMENTS:
    call STRING_EXPRESSION
    call FUNCTION_COMMA
    call STRING_EXPRESSION
    call LENGTH_CLOSE
    call STACK_TOP
    ld b, (cd)
    inc cd
    ld ef, 0x0B01
    call COPY_STRING_BYTES
    ld (ef), 0x00
    call STACK_POP
    call STACK_TOP
    ld b, (cd)
    ld 0x0E00, b
    inc cd
    ld ef, 0x0E01
    call COPY_STRING_BYTES
    ld (ef), 0x00
    call STACK_POP
    pop ef
    push gh
    ld gh, ef
    ld cd, 0x0E00
    ld b, (cd)
    ld a, 0x00
    cmp ab, gh
    bcc STRING_INSTR_MISSING
    clc
    add cd, gh
STRING_INSTR_CANDIDATE:
    push cd
    ld ef, 0x0B01
STRING_INSTR_COMPARE:
    ld b, (ef)
    cmp b, 0x00
    beq STRING_INSTR_FOUND
    ld a, (cd)
    cmp a, b
    bne STRING_INSTR_NEXT
    inc cd
    inc ef
    jmp STRING_INSTR_COMPARE
STRING_INSTR_NEXT:
    pop cd
    inc cd
    inc gh
    ld a, (cd)
    cmp a, 0x00
    bne STRING_INSTR_CANDIDATE
STRING_INSTR_MISSING:
    ld ab, 0x0000
    pop gh
    ret
STRING_INSTR_FOUND:
    pop cd
    ld ab, gh
    pop gh
    ret

; Shared token table: consumed by the ROM and the host codec.
TOKEN_TABLE:
KW_END:
    data 0x80, "END", 0x00
KW_FOR:
    data 0x81, "FOR", 0x00
KW_NEXT:
    data 0x82, "NEXT", 0x00
KW_DATA:
    data 0x83, "DATA", 0x00
KW_INPUT:
    data 0x84, "INPUT", 0x00
KW_DIM:
    data 0x85, "DIM", 0x00
KW_READ:
    data 0x86, "READ", 0x00
KW_LET:
    data 0x87, "LET", 0x00
KW_GOTO:
    data 0x88, "GOTO", 0x00
KW_RUN:
    data 0x89, "RUN", 0x00
KW_IF:
    data 0x8A, "IF", 0x00
KW_RESTORE:
    data 0x8B, "RESTORE", 0x00
KW_GOSUB:
    data 0x8C, "GOSUB", 0x00
KW_RETURN:
    data 0x8D, "RETURN", 0x00
KW_REM:
    data 0x8E, "REM", 0x00
KW_STOP:
    data 0x8F, "STOP", 0x00
KW_PRINT:
    data 0x90, "PRINT", 0x00
KW_LIST:
    data 0x91, "LIST", 0x00
KW_NEW:
    data 0x92, "NEW", 0x00
KW_SAVE:
    data 0x93, "SAVE", 0x00
KW_LOAD:
    data 0x94, "LOAD", 0x00
KW_BLOAD:
    data 0xA7, "BLOAD", 0x00
KW_BSAVE:
    data 0xA8, "BSAVE", 0x00
KW_QUIT:
    data 0x95, "QUIT", 0x00
KW_HELP:
    data 0x96, "HELP", 0x00
KW_THEN:
    data 0x97, "THEN", 0x00
KW_TO:
    data 0x98, "TO", 0x00
KW_STEP:
    data 0x99, "STEP", 0x00
KW_LEN:
    data 0x9A, "LEN", 0x00
KW_PEEK:
    data 0x9B, "PEEK", 0x00
KW_POKE:
    data 0x9C, "POKE", 0x00
KW_RND:
    data 0x9D, "RND", 0x00
KW_RANDOMIZE:
    data 0x9E, "RANDOMIZE", 0x00
KW_MID:
    data 0xA0, "MID$", 0x00
KW_LEFT:
    data 0xA1, "LEFT$", 0x00
KW_RIGHT:
    data 0xA2, "RIGHT$", 0x00
KW_CHR:
    data 0xA3, "CHR$", 0x00
KW_ASC:
    data 0xA4, "ASC", 0x00
KW_VAL:
    data 0xA5, "VAL", 0x00
KW_INSTR:
    data 0xA6, "INSTR", 0x00
KW_DEF:
    data 0xB6, "DEF", 0x00
KW_AT:
    data 0xB7, "AT", 0x00
VIDEO_KEYWORDS_START:
KW_SCREEN:
    data 0xA9, "SCREEN", 0x00
KW_CLS:
    data 0xAA, "CLS", 0x00
KW_COLOR:
    data 0xAB, "COLOR", 0x00
KW_PLOT:
    data 0xAC, "PLOT", 0x00
KW_LINE:
    data 0xAD, "LINE", 0x00
VIDEO_KEYWORDS_END:
TOKEN_TABLE_END:
    data 0x00

; GH source, EF scratch end. Entry tokenization is the only keyword scan.
TOKEN_NEXT:
    call SPACE
    cmp a, 0x00
    bne TOKEN_MORE
    call TOKEN_EMIT
    ret
TOKEN_MORE:
    cmp a, 0x22
    bne TOKEN_NOT_STRING
    jmp TOKEN_STRING
TOKEN_NOT_STRING:
    cmp a, 0x30
    bcc TOKEN_NOT_NUMBER
    cmp a, 0x3A
    bcs TOKEN_NOT_NUMBER
    jmp TOKEN_NUMBER
TOKEN_NOT_NUMBER:
    cmp a, 0x41
    bcc TOKEN_SYMBOL
    cmp a, 0x5B
    bcs TOKEN_SYMBOL
    ld cd, TOKEN_TABLE
TOKEN_KEYWORD:
    ld a, (cd)
    cmp a, 0x00
    beq TOKEN_WORD
    push cd
    inc cd
    call MATCH_TEXT
    pop cd
    cmp a, 0x00
    beq TOKEN_MATCHED
TOKEN_SKIP_KEYWORD:
    inc cd
    ld a, (cd)
    cmp a, 0x00
    bne TOKEN_SKIP_KEYWORD
    inc cd
    jmp TOKEN_KEYWORD
TOKEN_MATCHED:
    ld a, 0x01
    ld 0x00D3, a
    ld a, (cd)
    ld 0x00D4, a
    cmp a, 0xA7
    beq TOKEN_BINARY_KEYWORD
    cmp a, 0xA8
    bne TOKEN_KEYWORD_EMIT
TOKEN_BINARY_KEYWORD:
    ld a, 0x01
    ld 0x00D5, a
    ld a, 0x00
    ld 0x00D6, a
    ld a, (cd)
TOKEN_KEYWORD_EMIT:
    call TOKEN_EMIT
    ld 0x00D2, a
    cmp a, 0x83
    beq TOKEN_RAW
    cmp a, 0x8E
    bne TOKEN_NEXT
TOKEN_RAW:
    ld a, (gh)
    call TOKEN_EMIT
    inc gh
    cmp a, 0x00
    bne TOKEN_RAW
    ret
TOKEN_WORD:
    call WORD_CHAR
    beq TOKEN_NEXT
TOKEN_WORD_MORE:
    call TOKEN_EMIT
    inc gh
    ld a, 0x00
    ld 0x00D2, a
    ld 0x00D3, a
    ld 0x00D4, a
    jmp TOKEN_WORD
TOKEN_SYMBOL:
    cmp a, 0x2D
    bne TOKEN_SYMBOL_POSITIVE
    ld cd, 0x00D3
    ld a, (cd)
    cmp a, 0x00
    beq TOKEN_SYMBOL_MINUS
    push gh
    inc gh
    call SPACE
    cmp a, 0x30
    bcc TOKEN_MINUS_RESTORE
    cmp a, 0x3A
    bcs TOKEN_MINUS_RESTORE
    pop cd
    call NUMBER
    cmp ab, 0x8000
    bcc TOKEN_NEGATE
    beq TOKEN_NEGATE
    jmp OVERFLOW
TOKEN_NEGATE:
    call NEGATE
    push ab
    ld a, 0xB0
    jmp TOKEN_NUMBER_TAG
TOKEN_MINUS_RESTORE:
    pop gh
TOKEN_SYMBOL_MINUS:
    ld a, 0x2D
TOKEN_SYMBOL_POSITIVE:
    cmp a, 0x3F
    bne TOKEN_COMPARISON
    ld a, 0x90
    jmp TOKEN_SYMBOL_EMIT
TOKEN_COMPARISON:
    cmp a, 0x3C
    beq TOKEN_LESS
    cmp a, 0x3E
    bne TOKEN_ASCII
    ld a, (gh+0x01)
    cmp a, 0x3D
    bne TOKEN_ASCII
    ld a, 0xB5
    jmp TOKEN_PAIR
TOKEN_LESS:
    ld a, (gh+0x01)
    cmp a, 0x3D
    beq TOKEN_LE
    cmp a, 0x3E
    bne TOKEN_ASCII
    ld a, 0xB3
    jmp TOKEN_PAIR
TOKEN_LE:
    ld a, 0xB4
TOKEN_PAIR:
    inc gh
    jmp TOKEN_SYMBOL_EMIT
TOKEN_ASCII:
    call PEEK
    ld cd, TOKEN_PUNCTUATION
TOKEN_VALID_SYMBOL:
    ld b, (cd)
    cmp b, 0x00
    bne TOKEN_CHECK_SYMBOL
    jmp SYNTAX_ERROR
TOKEN_CHECK_SYMBOL:
    cmp a, b
    beq TOKEN_SYMBOL_EMIT
    inc cd
    jmp TOKEN_VALID_SYMBOL
TOKEN_SYMBOL_EMIT:
    push ab
    ld cd, 0x00D5
    ld b, (cd)
    cmp b, 0x02
    bcc TOKEN_BINARY_SYMBOL_DONE
    cmp a, 0x28
    bne TOKEN_BINARY_CLOSE
    ld cd, 0x00D6
    ld b, (cd)
    inc b
    ld (cd), b
    jmp TOKEN_BINARY_SYMBOL_DONE
TOKEN_BINARY_CLOSE:
    cmp a, 0x29
    bne TOKEN_BINARY_COMMA
    ld cd, 0x00D6
    ld b, (cd)
    dec b
    ld (cd), b
    jmp TOKEN_BINARY_SYMBOL_DONE
TOKEN_BINARY_COMMA:
    cmp a, 0x2C
    bne TOKEN_BINARY_SYMBOL_DONE
    ld cd, 0x00D6
    ld b, (cd)
    cmp b, 0x00
    bne TOKEN_BINARY_SYMBOL_DONE
    ld cd, 0x00D5
    ld b, (cd)
    inc b
    ld (cd), b
TOKEN_BINARY_SYMBOL_DONE:
    pop ab
    ld b, 0x01
    cmp a, 0x29
    bne TOKEN_SYMBOL_UNARY
    ld b, 0x00
TOKEN_SYMBOL_UNARY:
    ld 0x00D3, b
    call TOKEN_EMIT
    inc gh
    ld a, 0x00
    ld 0x00D2, a
    jmp TOKEN_NEXT
TOKEN_NUMBER:
    call NUMBER
    push ab
    cmp ab, 0x8000
    bcc TOKEN_NUMBER_RANGE_OK
    ld cd, 0x00D2
    ld a, (cd)
    cmp a, 0x88
    beq TOKEN_NUMBER_RANGE_OK
    cmp a, 0x8B
    beq TOKEN_NUMBER_RANGE_OK
    cmp a, 0x8C
    beq TOKEN_NUMBER_RANGE_OK
    cmp a, 0x97
    beq TOKEN_NUMBER_RANGE_OK
    ld cd, 0x00D4
    ld a, (cd)
    cmp a, 0x9B
    beq TOKEN_NUMBER_RANGE_OK
    cmp a, 0x9C
    beq TOKEN_NUMBER_RANGE_OK
    ld cd, 0x00D5
    ld a, (cd)
    cmp a, 0x03
    bcs TOKEN_NUMBER_RANGE_OK
    jmp OVERFLOW
TOKEN_NUMBER_RANGE_OK:
    ld cd, 0x00D5
    ld a, (cd)
    cmp a, 0x03
    bcc TOKEN_NUMBER_STANDARD
    ld a, 0xB2
    jmp TOKEN_NUMBER_TAG
TOKEN_NUMBER_STANDARD:
    ld cd, 0x00D2
    ld a, (cd)
    cmp a, 0x88
    beq TOKEN_REFERENCE
    cmp a, 0x8B
    beq TOKEN_REFERENCE
    cmp a, 0x8C
    beq TOKEN_REFERENCE
    cmp a, 0x97
    beq TOKEN_REFERENCE
    ld a, 0xB0
    jmp TOKEN_NUMBER_TAG
TOKEN_REFERENCE:
    ld a, 0xB2
TOKEN_NUMBER_TAG:
    call TOKEN_EMIT
    pop ab
    push a
    ld a, b
    call TOKEN_EMIT
    pop a
    call TOKEN_EMIT
    ld a, 0x00
    ld 0x00D2, a
    ld 0x00D3, a
    ld 0x00D4, a
    jmp TOKEN_NEXT
TOKEN_STRING:
    ld a, 0xB1
    call TOKEN_EMIT
    ld cd, ef
    ld a, 0x00
    call TOKEN_EMIT
    inc gh
    ld b, 0x00
TOKEN_STRING_BYTE:
    ld a, (gh)
    inc gh
    cmp a, 0x00
    bne TOKEN_STRING_NOT_END
    jmp UNTERMINATED_STRING
TOKEN_STRING_NOT_END:
    cmp a, 0x22
    beq TOKEN_STRING_END
    cmp b, 0xFF
    bne TOKEN_STRING_FITS
    jmp STRING_TOO_LONG
TOKEN_STRING_FITS:
    cmp a, 0x09
    beq TOKEN_STRING_VALID
    cmp a, 0x20
    bcs TOKEN_STRING_HIGH
    jmp INVALID_STRING_CHARACTER
TOKEN_STRING_HIGH:
    cmp a, 0x7F
    bcc TOKEN_STRING_VALID
    jmp INVALID_STRING_CHARACTER
TOKEN_STRING_VALID:
    call TOKEN_EMIT
    inc b
    jmp TOKEN_STRING_BYTE
TOKEN_STRING_END:
    push gh
    push ab
    ld ab, 0x00ED
    ld a, (ab)
    cmp a, 0x00
    bne TOKEN_STRING_LENGTH_DONE
    cmp cd, 0x0E00
    bcs TOKEN_STRING_LENGTH_STORE
    ld ab, 0x00EC
    ld a, (ab)
    cmp a, 0x00
    beq TOKEN_STRING_LENGTH_STORE
    sec
    sub cd, 0x0900
    ld ab, 0x00E8
    ld h, (ab)
    ld g, (ab+0x01)
    clc
    add cd, gh

TOKEN_STRING_LENGTH_STORE:
    pop ab
    ld (cd), b
    jmp TOKEN_STRING_LENGTH_END
TOKEN_STRING_LENGTH_DONE:
    pop ab
TOKEN_STRING_LENGTH_END:
    pop gh
    ld cd, 0x00D5
    ld a, (cd)
    cmp a, 0x01
    bne TOKEN_STRING_CONTEXT_DONE
    ld a, 0x02
    ld (cd), a
TOKEN_STRING_CONTEXT_DONE:
    ld a, 0x00
    ld 0x00D2, a
    ld 0x00D3, a
    ld 0x00D4, a
    jmp TOKEN_NEXT
TOKEN_EMIT:
    push ab
    push cd
    ld cd, 0x00ED
    ld b, (cd)
    cmp b, 0x00
    beq TOKEN_EMIT_STORE
    cmp ef, 0x88FA
    bcc TOKEN_EMIT_COUNT
    jmp INPUT_TOO_LONG
TOKEN_EMIT_COUNT:
    inc ef
    pop cd
    pop ab
    ret
TOKEN_EMIT_STORE:
    ld cd, 0x00EC
    ld b, (cd)
    cmp b, 0x00
    bne TOKEN_EMIT_STAGED
    cmp ef, 0x0E00
    bcc TOKEN_EMIT_BYTE
    ld cd, 0x00E6
    ld b, (cd)
    ld a, (cd+0x01)
    cmp ab, 0x0000
    bne TOKEN_SPILL
    jmp INPUT_TOO_LONG
TOKEN_SPILL:
    push gh
    ld cd, 0x0086
    ld h, (cd)
    ld g, (cd+0x01)
    clc
    add gh, 0x0004
    ld 0x00E8, h
    ld 0x00E9, g
    ld ef, gh
    clc
    add ef, 0x0500
    cmp ef, 0x7FFE
    bcc TOKEN_SPILL_FITS
    jmp INPUT_TOO_LONG
TOKEN_SPILL_FITS:
    ld cd, 0x0900
TOKEN_SPILL_COPY:
    ld a, (cd)
    ld (gh), a
    inc cd
    inc gh
    cmp gh, ef
    bne TOKEN_SPILL_COPY
    pop gh
    ld a, 0x01
    ld 0x00EC, a
TOKEN_EMIT_STAGED:
    cmp ef, 0x7FFE
    bcc TOKEN_EMIT_BYTE
    jmp INPUT_TOO_LONG
TOKEN_EMIT_BYTE:
    pop cd
    pop ab
    ld (ef), a
    inc ef
    ret
TOKEN_PUNCTUATION:
    data "+-*/()=<>;,", 0x00
; Return Z if the entry character is not part of a word; preserve A.
WORD_CHAR:
    call PEEK
    cmp a, 0x24
    beq WORD_YES
    cmp a, 0x30
    bcc WORD_NO
    cmp a, 0x3A
    bcc WORD_YES
    cmp a, 0x41
    bcc WORD_NO
    cmp a, 0x5B
    bcc WORD_YES
WORD_NO:
    cmp a, a
    ret
WORD_YES:
    cmp a, 0x00
    ret
; Skip a complete token stream without mistaking binary zeroes for EOL.
TOKEN_LINE_END:
    ld a, (gh)
    inc gh
    cmp a, 0x00
    beq TOKEN_LINE_DONE
    cmp a, 0xB0
    beq TOKEN_LINE_WORD
    cmp a, 0xB2
    beq TOKEN_LINE_WORD
    cmp a, 0xB1
    beq TOKEN_LINE_STRING
    cmp a, 0x83
    beq TOKEN_LINE_RAW
    cmp a, 0x8E
    beq TOKEN_LINE_RAW
    jmp TOKEN_LINE_END
TOKEN_LINE_WORD:
    inc gh
    inc gh
    jmp TOKEN_LINE_END
TOKEN_LINE_STRING:
    ld b, (gh)
    ld a, 0x00
    inc ab
    clc
    add gh, ab
    jmp TOKEN_LINE_END
TOKEN_LINE_RAW:
    ld a, (gh)
    inc gh
    cmp a, 0x00
    bne TOKEN_LINE_RAW
TOKEN_LINE_DONE:
    ret
; LIST/SAVE render canonical source directly to the selected output device.
DETOKENIZE:
    ld a, (gh)
    cmp a, 0x00
    bne DETOKEN_MORE
    ret
DETOKEN_MORE:
    cmp a, 0xB0
    beq DETOKEN_NUMBER
    cmp a, 0xB2
    beq DETOKEN_NUMBER
    inc gh
    cmp a, 0xB1
    beq DETOKEN_STRING
    cmp a, 0xB3
    bcc DETOKEN_NOT_OPERATOR
    cmp a, 0xB6
    bcs DETOKEN_NOT_OPERATOR
    push a
    cmp a, 0xB5
    beq DETOKEN_GREATER
    ld a, 0x3C
    jmp DETOKEN_OPERATOR_FIRST
DETOKEN_GREATER:
    ld a, 0x3E
DETOKEN_OPERATOR_FIRST:
    call PUTCHAR
    pop a
    cmp a, 0xB3
    beq DETOKEN_NOT_EQUAL
    ld a, 0x3D
    jmp DETOKEN_CHAR
DETOKEN_NOT_EQUAL:
    ld a, 0x3E
    jmp DETOKEN_CHAR
DETOKEN_NOT_OPERATOR:
    cmp a, 0x80
    bcs DETOKEN_KEYWORD
DETOKEN_CHAR:
    call PUTCHAR
    jmp DETOKENIZE
DETOKEN_NUMBER:
    ld b, a
    call NUMBER
    push ab
    ld a, (gh)
    pop ab
    ; Line references are unsigned; ordinary constants are signed.
    push gh
    dec gh
    dec gh
    dec gh
    ld c, (gh)
    pop gh
    cmp c, 0xB2
    beq DETOKEN_LINE_NUMBER
    call PRINT_NUM
    jmp DETOKENIZE
DETOKEN_LINE_NUMBER:
    call PRINT_LINE
    jmp DETOKENIZE
DETOKEN_STRING:
    ld b, (gh)
    inc gh
    ld a, 0x22
    call PUTCHAR
DETOKEN_STRING_BYTE:
    cmp b, 0x00
    beq DETOKEN_STRING_END
    ld a, (gh)
    call PUTCHAR
    inc gh
    dec b
    jmp DETOKEN_STRING_BYTE
DETOKEN_STRING_END:
    ld a, 0x22
    jmp DETOKEN_CHAR
DETOKEN_KEYWORD:
    ld b, a
    cmp a, 0x97
    bcc DETOKEN_LOOKUP
    cmp a, 0x9A
    bcs DETOKEN_LOOKUP
    ld a, 0x20
    call PUTCHAR
DETOKEN_LOOKUP:
    ld cd, TOKEN_TABLE
DETOKEN_SEARCH:
    ld a, (cd)
    inc cd
    cmp a, b
    beq DETOKEN_FOUND
DETOKEN_SKIP:
    ld a, (cd)
    inc cd
    cmp a, 0x00
    bne DETOKEN_SKIP
    jmp DETOKEN_SEARCH
DETOKEN_FOUND:
    push b
    call PUTS
    pop b
    cmp b, 0x83
    beq DETOKEN_RAW
    cmp b, 0x8E
    beq DETOKEN_RAW
    cmp b, 0x9C
    beq DETOKEN_KEYWORD_SPACE
    cmp b, 0x9E
    beq DETOKEN_KEYWORD_SPACE
    cmp b, 0xB6
    beq DETOKEN_KEYWORD_SPACE
VIDEO_SPACING_START:
    cmp b, 0xA9
    bcc VIDEO_SPACING_END
    cmp b, 0xAE
    bcc DETOKEN_KEYWORD_SPACE
VIDEO_SPACING_END:
    cmp b, 0xA7
    beq DETOKEN_KEYWORD_SPACE
    cmp b, 0xA8
    beq DETOKEN_KEYWORD_SPACE
    cmp b, 0x9A
    bcc TOKEN_LONG_0_0
    jmp DETOKENIZE
TOKEN_LONG_0_0:
DETOKEN_KEYWORD_SPACE:
    ld a, (gh)
    cmp a, 0x00
    bne TOKEN_LONG_0_1
    jmp DETOKENIZE
TOKEN_LONG_0_1:
    ld a, 0x20
    jmp DETOKEN_CHAR
DETOKEN_RAW:
    ld cd, gh
    jmp PUTS

; Streaming entry: lexical items start at 0200 and spill to A000. Token bytes
; start at 0900 and large numbered lines spill beyond the old end marker.
; ED discards token bytes during LOAD validation; EE tracks an unfinished line.
READ_TOKEN_LINE:
    call SC_EDIT_START
    ld a, 0x00
    ld 0x00E6, a
    ld 0x00E7, a
    ld 0x00EC, a
    ld 0x00ED, a
    ld 0x00D2, a
    ld 0x00D4, a
    ld 0x00D5, a
    ld 0x00D6, a
    ld 0x00D9, a
    ld a, 0x01
    ld 0x00EE, a
    ld 0x00D3, a
    ld cd, 0x00A6
    ld a, (cd)
    cmp a, 0x00
    beq STREAM_INIT
    ld cd, 0x00AC
    ld a, (cd)
    cmp a, 0x00
    bne STREAM_INIT
    ld a, 0x01
    ld 0x00ED, a
STREAM_INIT:
    ld ef, 0x0900
    ld 0x00E8, f
    ld 0x00E9, e
    call STREAM_GET
    call STREAM_SPACE
    cmp a, 0x30
    bcc STREAM_BODY
    cmp a, 0x3A
    bcs STREAM_BODY
    call STREAM_BUFFER_START
STREAM_LINE_DIGIT:
    call STREAM_BUFFER
    call STREAM_GET
    cmp a, 0x30
    bcc STREAM_LINE_NUMBER
    cmp a, 0x3A
    bcc STREAM_LINE_DIGIT
STREAM_LINE_NUMBER:
    call STREAM_BUFFER_END
    call NUMBER
    cmp ab, 0x0000
    bne STREAM_LINE_VALID
    jmp INVALID_LINE_NUMBER
STREAM_LINE_VALID:
    ld 0x00E6, b
    ld 0x00E7, a
STREAM_BODY:
    call STREAM_CURRENT
    call STREAM_SPACE
    cmp a, 0x00
    bne STREAM_ITEM
    call TOKEN_EMIT
    ret
STREAM_ITEM:
    call STREAM_BUFFER_START
    cmp a, 0x22
    beq STREAM_STRING
    cmp a, 0x2D
    beq STREAM_MINUS
    cmp a, 0x3C
    beq STREAM_COMPARE
    cmp a, 0x3E
    beq STREAM_COMPARE
    call STREAM_IS_WORD
    bne STREAM_WORD
    call STREAM_BUFFER
    call STREAM_GET
    jmp STREAM_ENCODE
STREAM_WORD:
    call STREAM_BUFFER
    call STREAM_GET
    call STREAM_IS_WORD
    bne STREAM_WORD
    jmp STREAM_ENCODE
STREAM_MINUS:
    call STREAM_BUFFER
    call STREAM_GET
    push cd
    ld cd, 0x00D3
    ld a, (cd)
    pop cd
    cmp a, 0x00
    beq STREAM_ENCODE
    call STREAM_CURRENT
    call STREAM_SPACE
    cmp a, 0x30
    bcc STREAM_ENCODE
    cmp a, 0x3A
    bcs STREAM_ENCODE
    jmp STREAM_WORD
STREAM_COMPARE:
    call STREAM_BUFFER
    call STREAM_GET
    cmp a, 0x3D
    beq STREAM_COMPARE_SECOND
    cmp a, 0x3E
    bne STREAM_ENCODE
    ld gh, 0x0200
    ld b, (gh)
    cmp b, 0x3C
    bne STREAM_ENCODE
STREAM_COMPARE_SECOND:
    call STREAM_BUFFER
    call STREAM_GET
    jmp STREAM_ENCODE
STREAM_STRING:
    ld b, 0x01
    ld 0x00D9, b
    call STREAM_BUFFER
STREAM_STRING_MORE:
    call STREAM_GET
    cmp a, 0x00
    bne STREAM_STRING_BYTE
    jmp UNTERMINATED_STRING
STREAM_STRING_BYTE:
    call STREAM_BUFFER
    cmp a, 0x22
    bne STREAM_STRING_MORE
    ld b, 0x00
    ld 0x00D9, b
    call STREAM_GET
STREAM_ENCODE:
    call STREAM_BUFFER_END
    call TOKEN_NEXT
    dec ef
    ld cd, 0x00D2
    ld a, (cd)
    cmp a, 0x83
    beq STREAM_RAW
    cmp a, 0x8E
    beq STREAM_RAW
    jmp STREAM_BODY
STREAM_RAW:
    call STREAM_CURRENT
    call TOKEN_EMIT
    cmp a, 0x00
    beq STREAM_DONE
    call STREAM_GET
    jmp STREAM_RAW
STREAM_DONE:
    ret
STREAM_BUFFER_START:
    ld cd, 0x0200
    ld 0x00DA, d
    ld 0x00DB, c
    ret
STREAM_BUFFER_END:
    ld a, 0x00
    call STREAM_BUFFER
    ld cd, 0x00DA
    ld h, (cd)
    ld g, (cd+0x01)
    ret
STREAM_BUFFER:
    cmp cd, 0x0300
    bne STREAM_BUFFER_ROOM
    push ab
    push ef
    push gh
    ld gh, 0x0200
    ld ef, 0xA000
STREAM_BUFFER_SPILL:
    ld b, (gh)
    ld (ef), b
    inc gh
    inc ef
    cmp gh, 0x0300
    bne STREAM_BUFFER_SPILL
    ld cd, ef
    ld gh, 0xA000
    ld 0x00DA, h
    ld 0x00DB, g
    pop gh
    pop ef
    pop ab
STREAM_BUFFER_ROOM:
    cmp cd, 0xA400
    bcc STREAM_BUFFER_FITS
    jmp INPUT_TOO_LONG
STREAM_BUFFER_FITS:
    ld (cd), a
    inc cd
    ret
STREAM_SPACE:
    cmp a, 0x20
    beq STREAM_SKIP
    cmp a, 0x09
    bne STREAM_DONE
STREAM_SKIP:
    call STREAM_GET
    jmp STREAM_SPACE
STREAM_CURRENT:
    push cd
    ld cd, 0x00D8
    ld a, (cd)
    pop cd
    ret
STREAM_GET:
    call GETCHAR
    cmp a, 0x0D
    beq STREAM_GET
    cmp a, 0x0A
    beq STREAM_END
    cmp a, 0xFF
    beq STREAM_END
    cmp a, 0x09
    beq STREAM_GOT
    cmp a, 0x20
    bcc STREAM_BAD
    cmp a, 0x7F
    bcc STREAM_GOT
STREAM_BAD:
    ld cd, 0x00D9
    ld a, (cd)
    cmp a, 0x00
    beq STREAM_BAD_ASCII
    jmp INVALID_STRING_CHARACTER
STREAM_BAD_ASCII:
    jmp BAD_INPUT_CHARACTER
STREAM_END:
    ld a, 0x00
    ld 0x00EE, a
STREAM_GOT:
    ld 0x00D8, a
    ret
STREAM_IS_WORD:
    cmp a, 0x24
    beq STREAM_WORD_YES
    cmp a, 0x30
    bcc STREAM_WORD_NO
    cmp a, 0x3A
    bcc STREAM_WORD_YES
    cmp a, 0x41
    bcc STREAM_WORD_NO
    cmp a, 0x5B
    bcc STREAM_WORD_YES
    cmp a, 0x61
    bcc STREAM_WORD_NO
    cmp a, 0x7B
    bcc STREAM_WORD_YES
STREAM_WORD_NO:
    cmp a, a
    ret
STREAM_WORD_YES:
    cmp a, 0x00
    ret
; The complete large line is already staged after the old chain. Remove a
; replaced record, then rotate the appended record into sorted position.
EDIT_STAGED:
    push gh
    ld gh, cd
    ld ef, 0x00E6
    ld a, (ef)
    ld (gh+0x02), a
    ld a, (ef+0x01)
    ld (gh+0x03), a
    ld ef, 0x00E4
    ld b, (ef)
    ld a, (ef+0x01)
    clc
    add cd, ab
    ld 0x00EA, d
    ld 0x00EB, c
    ld cd, 0x00E0
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, 0x00E2
    ; Load a pointer through a separate pair to avoid changing its base early.
    ld cd, 0x00E2
    ld b, (cd)
    ld a, (cd+0x01)
    ld cd, ab
    ld ab, ef
    sec
    sub ab, cd
    clc
    add gh, ab
    ld 0x00E2, h
    ld 0x00E3, g
    ld gh, 0x00EA
    ld b, (gh)
    ld a, (gh+0x01)
EDIT_STAGE_SLIDE:
    cmp cd, ab
    beq EDIT_STAGE_ROTATE
    ld g, (cd)
    ld (ef), g
    inc cd
    inc ef
    jmp EDIT_STAGE_SLIDE
EDIT_STAGE_ROTATE:
    pop gh
    ld (gh), 0x00
    inc gh
    ld (gh), 0x00
    ld cd, 0x00E0
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, 0x00E2
    ld h, (cd)
    ld g, (cd+0x01)
    call REVERSE_BYTES
    ld cd, 0x00E2
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, 0x0086
    ld h, (cd)
    ld g, (cd+0x01)
    call REVERSE_BYTES
    ld cd, 0x00E0
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, 0x0086
    ld h, (cd)
    ld g, (cd+0x01)
    call REVERSE_BYTES
    jmp EDIT_LINKS
REVERSE_BYTES:
    cmp ef, gh
    bcs REVERSE_DONE
    dec gh
    cmp ef, gh
    bcs REVERSE_DONE
    ld a, (ef)
    ld b, (gh)
    ld (ef), b
    ld (gh), a
    inc ef
    jmp REVERSE_BYTES
REVERSE_DONE:
    ret

; Video commands in the optional native BASIC ROM.
; Reuses EXPR, EOL, BINARY_COMMA, REPORT_ERROR; no new RAM scratch.
VIDEO_SCREEN:
    ld ef, 0x0002
    call VIDEO_ARGUMENT
    push ab
    call EOL
    pop ab
    ld 0xFF30, b
    ret
VIDEO_COLOR:
    ld ef, 0x0010
    call VIDEO_ARGUMENT
    push ab
    call BINARY_COMMA
    call VIDEO_ARGUMENT
    push ab
    call EOL
    pop ab
    pop cd
    ld 0xFF34, d
    ld 0xFF35, b
    ret
VIDEO_PLOT:
    ld ef, 0x00A0
    call VIDEO_ARGUMENT
    push ab
    call BINARY_COMMA
    ld ef, 0x0060
    call VIDEO_ARGUMENT
    push ab
    call EOL
    ld cd, 0xFF30
    ld a, (cd)
    and a, 0x01
    cmp a, 0x01
    beq VIDEO_PLOT_MODE_OK
    jmp VIDEO_ILLEGAL
VIDEO_PLOT_MODE_OK:
    pop ab
    pop cd
    ld 0xFF36, d
    ld 0xFF37, b
    ld a, 0x01
    ld 0xFF3A, a
    ret
VIDEO_LINE:
    ld ef, 0x00A0
    call VIDEO_ARGUMENT
    ld 0x03D2, b
    call BINARY_COMMA
    ld ef, 0x0060
    call VIDEO_ARGUMENT
    ld 0x03D3, b
    call BINARY_COMMA
    ld ef, 0x00A0
    call VIDEO_ARGUMENT
    ld 0x03D4, b
    call BINARY_COMMA
    ld ef, 0x0060
    call VIDEO_ARGUMENT
    ld 0x03D5, b
    call EOL
    ld cd, 0xFF30
    ld a, (cd)
    and a, 0x01
    cmp a, 0x01
    beq LINE_MODE_OK
    jmp VIDEO_ILLEGAL
LINE_MODE_OK:
    ld cd, 0x03D4
    ld b, (cd)
    ld a, 0x00
    ld ef, 0x03D2
    ld f, (ef)
    ld e, 0x00
    sec
    sub ab, ef
    cmp a, 0x80
    bcc LINE_DX_POSITIVE
    call NEGATE
    ld c, 0xFF
    jmp LINE_DX_STORE
LINE_DX_POSITIVE:
    ld c, 0x01
LINE_DX_STORE:
    ld 0x03DC, c
    ld 0x03D6, b
    ld 0x03D7, a
    ld cd, 0x03D5
    ld b, (cd)
    ld a, 0x00
    ld ef, 0x03D3
    ld f, (ef)
    ld e, 0x00
    sec
    sub ab, ef
    cmp a, 0x80
    bcc LINE_DY_POSITIVE
    call NEGATE
    ld c, 0xFF
    jmp LINE_DY_STORE
LINE_DY_POSITIVE:
    ld c, 0x01
LINE_DY_STORE:
    ld 0x03DD, c
    ld 0x03D8, b
    ld 0x03D9, a
    ld ef, ab
    ld cd, 0x03D6
    ld b, (cd)
    ld a, (cd+0x01)
    sec
    sub ab, ef
    ld 0x03DA, b
    ld 0x03DB, a
LINE_DRAW:
    ld cd, 0x03D2
    ld b, (cd)
    ld 0xFF36, b
    ld b, (cd+0x01)
    ld 0xFF37, b
    ld b, 0x01
    ld 0xFF3A, b
    ld b, (cd)
    ld a, (cd+0x02)
    cmp b, a
    bne LINE_CONTINUE
    ld b, (cd+0x01)
    ld a, (cd+0x03)
    cmp b, a
    bne LINE_CONTINUE
    jmp LINE_DONE
LINE_CONTINUE:
    ld cd, 0x03DA
    ld b, (cd)
    ld a, (cd+0x01)
    shl ab, 0x01
    ld 0x03DE, b
    ld 0x03DF, a
    ld cd, 0x03D8
    ld f, (cd)
    ld e, (cd+0x01)
    call ADD_SIGNED
    cmp ab, 0x0000
    beq LINE_NO_X
    cmp a, 0x80
    bcs LINE_NO_X
    ld cd, 0x03DA
    ld b, (cd)
    ld a, (cd+0x01)
    ld ef, 0x03D8
    ld f, (ef)
    ld e, (ef+0x01)
    call SUB_SIGNED
    ld (cd), b
    ld (cd+0x01), a
    ld cd, 0x03D2
    ld a, (cd)
    ld ef, 0x03DC
    ld b, (ef)
    clc
    add a, b
    ld (cd), a
LINE_NO_X:
    ld cd, 0x03DE
    ld b, (cd)
    ld a, (cd+0x01)
    ld ef, 0x03D6
    ld f, (ef)
    ld e, (ef+0x01)
    call SUB_SIGNED
    cmp a, 0x80
    bcs LINE_DO_Y
    jmp LINE_DRAW
LINE_DO_Y:
    ld cd, 0x03DA
    ld b, (cd)
    ld a, (cd+0x01)
    ld ef, 0x03D6
    ld f, (ef)
    ld e, (ef+0x01)
    call ADD_SIGNED
    ld (cd), b
    ld (cd+0x01), a
    ld cd, 0x03D3
    ld a, (cd)
    ld ef, 0x03DD
    ld b, (ef)
    clc
    add a, b
    ld (cd), a
    jmp LINE_DRAW
LINE_DONE:
    ret
VIDEO_ARGUMENT:
    call EXPR
    cmp ab, ef
    bcc VIDEO_ARGUMENT_OK
VIDEO_ILLEGAL:
    ld cd, VIDEO_ILLEGAL_TEXT
    jmp REPORT_ERROR
VIDEO_ARGUMENT_OK:
    ret
VIDEO_ILLEGAL_TEXT:
    data "? ILLEGAL QUANTITY", 0x00
VIDEO_CLS:
    call EOL
    ld cd, 0xFF30
    ld a, (cd)
    and a, 0x01
    ld b, a
    ld a, 0x00
    ld 0xFF31, a
    ld 0xFF32, a
    cmp b, 0x00
    beq VIDEO_CLS_TEXT
    ld ef, 0x0F00
VIDEO_CLS_PICTURE_LOOP:
    ld 0xFF33, a
    dec ef
    bne VIDEO_CLS_PICTURE_LOOP
    ret
VIDEO_CLS_TEXT:
    call SC_RESET
    ld cd, 0xFF35
    ld a, (cd)
    shl a, 0x04
    ld b, a
    dec cd
    ld a, (cd)
    or a, b
    ld b, a
    ld ef, 0x03C0
VIDEO_CLS_TEXT_LOOP:
    ld a, 0x20
    ld 0xFF33, a
    ld 0xFF33, b
    dec ef
    bne VIDEO_CLS_TEXT_LOOP
    ret
VIDEO_END:

; Prototype scratch in unused system RAM: F0/F1 byte offset, F2 column,
; F3 route (0 screen, nonzero serial). Host sets F3 before entry.
SC_RESET:
    push a
    ld a, 0x00
    ld 0x00F0, a
    ld 0x00F1, a
    ld 0x00F2, a
    pop a
    ret
SC_PROMPT:
    ld cd, 0x00F3
    ld a, (cd)
    cmp a, 0x00
    bne SC_PROMPT_DONE
    ld 0xFF30, a
SC_PROMPT_DONE:
    ret
SC_OUTPUT:
    push ab
    push cd
    push ef
    push gh
    ld cd, 0x00F3
    ld b, (cd)
    cmp b, 0x00
    beq SC_SCREEN
    ld 0xFF02, a
    jmp SC_RETURN
SC_SCREEN:
    ld cd, 0x00F9
    ld b, (cd)
    cmp b, 0x02
    beq SC_RETURN
    ld cd, 0xFF31
    ld h, (cd)
    ld g, (cd+0x01)
    push gh
    dec cd
    ld b, (cd)
    and b, 0x01
    push b
    ld b, 0x00
    ld 0xFF30, b
    ld cd, 0x00F0
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, 0xFF35
    ld b, (cd)
    shl b, 0x04
    dec cd
    ld h, (cd)
    or b, h
    ld cd, 0x00F2
    ld h, (cd)
    cmp a, 0x0D
    beq SC_FINISH
    cmp a, 0x0A
    beq SC_NEWLINE
    cmp a, 0x09
    bne SC_PRINTABLE
    ld a, 0x20
SC_PRINTABLE:
    cmp a, 0x20
    bcc SC_FINISH
    call SC_CELL
    jmp SC_FINISH
SC_NEWLINE:
    ld a, 0x20
    call SC_CELL
    cmp h, 0x00
    bne SC_NEWLINE
SC_FINISH:
    ld 0x00F0, f
    ld 0x00F1, e
    ld 0x00F2, h
    pop b
    ld 0xFF30, b
    pop ef
    call SC_POINTER
SC_RETURN:
    pop gh
    pop ef
    pop cd
    pop ab
    ret
SC_POINTER:
    ld 0xFF31, f
    ld 0xFF32, e
    ret
SC_CELL:
    call SC_POINTER
    ld 0xFF33, a
    ld 0xFF33, b
    inc ef
    inc ef
    inc h
    cmp h, 0x28
    bne SC_CELL_DONE
    ld h, 0x00
    cmp ef, 0x0780
    bcc SC_CELL_DONE
    push ab
    push gh
    ld ef, 0x0000
SC_SCROLL:
    push ef
    ld ab, 0x0050
    clc
    add ef, ab
    call SC_POINTER
    ld cd, 0xFF33
    ld a, (cd)
    pop ef
    call SC_POINTER
    ld 0xFF33, a
    inc ef
    cmp ef, 0x0730
    bcc SC_SCROLL
    pop gh
    pop ab
    push ef
    call SC_POINTER
    push a
    ld a, 0x20
SC_CLEAR_ROW:
    ld 0xFF33, a
    ld 0xFF33, b
    inc ef
    inc ef
    cmp ef, 0x0780
    bcc SC_CLEAR_ROW
    pop a
    pop ef
SC_CELL_DONE:
    ret
SC_END:
; F4 is the replay flag, F5/F6 the next raw byte, F7/F8 the end.
; F9 marks an active interactive editor for a renderer cursor. A400-A7FF
; holds the raw editable line. File input and serial input bypass this path.
SC_EDIT_START:
    push cd
    push ef
    ld cd, 0x00F3
    ld a, (cd)
    cmp a, 0x00
    beq SC_EDIT_SCREEN
    jmp SC_EDIT_DONE
SC_EDIT_SCREEN:
    ld a, (cd+0x06)
    cmp a, 0x02
    bne SC_EDIT_NOT_PRELOAD
    jmp SC_EDIT_DONE
SC_EDIT_NOT_PRELOAD:
    ld cd, 0x00A6
    ld a, (cd)
    cmp a, 0x00
    bne SC_EDIT_DONE
    ld a, 0x01
    ld 0x00F9, a
    ld ef, 0xA400
SC_EDIT_LOOP:
    call GETCHAR_RAW
    cmp a, 0x08
    beq SC_EDIT_BACKSPACE
    cmp a, 0x7F
    beq SC_EDIT_BACKSPACE
    cmp a, 0x0D
    beq SC_EDIT_ENTER
    cmp a, 0x0A
    beq SC_EDIT_ENTER
    cmp a, 0x09
    beq SC_EDIT_STORE
    cmp a, 0x20
    bcc SC_EDIT_LOOP
    cmp a, 0x7F
    bcs SC_EDIT_LOOP
SC_EDIT_STORE:
    cmp ef, 0xA800
    bcs SC_EDIT_OVERFLOW
    ld (ef), a
    inc ef
    call SC_OUTPUT
    jmp SC_EDIT_LOOP
SC_EDIT_OVERFLOW:
    ld a, 0x00
    ld 0x00F9, a
    call NEWLINE
    jmp INPUT_TOO_LONG
SC_EDIT_BACKSPACE:
    cmp ef, 0xA400
    beq SC_EDIT_LOOP
    dec ef
    ld cd, 0x00F0
    ld b, (cd)
    ld a, (cd+0x01)
    cmp ab, 0x0000
    beq SC_EDIT_LOOP
    call SC_REWIND
    ld a, 0x20
    call SC_OUTPUT
    call SC_REWIND
    jmp SC_EDIT_LOOP
SC_EDIT_ENTER:
    call SC_OUTPUT
    ld a, 0x00
    ld 0x00F9, a
    ld a, 0x01
    ld 0x00F4, a
    ld a, 0x00
    ld 0x00F5, a
    ld a, 0xA4
    ld 0x00F6, a
    ld 0x00F7, f
    ld 0x00F8, e
SC_EDIT_DONE:
    pop ef
    pop cd
    ret
SC_GET:
    push cd
    ld cd, 0x00F4
    ld a, (cd)
    cmp a, 0x00
    bne SC_GET_REPLAY
    pop cd
    jmp GETCHAR_RAW
SC_GET_REPLAY:
    push ef
    ld cd, 0x00F5
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, 0x00F7
    ld b, (cd)
    ld a, (cd+0x01)
    cmp ef, ab
    beq SC_GET_END
    ld a, (ef)
    inc ef
    ld 0x00F5, f
    ld 0x00F6, e
    jmp SC_GET_RETURN
SC_GET_END:
    ld a, 0x00
    ld 0x00F4, a
    ld a, 0x0A
SC_GET_RETURN:
    pop ef
    pop cd
    ret
SC_REWIND:
    ld cd, 0x00F0
    ld b, (cd)
    ld a, (cd+0x01)
    dec ab
    dec ab
    ld (cd), b
    ld (cd+0x01), a
    ld cd, 0x00F2
    ld a, (cd)
    cmp a, 0x00
    bne SC_REWIND_COL
    ld a, 0x28
SC_REWIND_COL:
    dec a
    ld (cd), a
    ret
; Screen INPUT reveals the question, then restores the picture on success.
; FA records saved mode, FB/FC the hardware VRAM pointer.
SC_INPUT_BEGIN:
    push ab
    push cd
    ld a, 0x00
    ld 0x00FA, a
    ld cd, 0x00F3
    ld a, (cd)
    cmp a, 0x00
    bne SC_INPUT_BEGIN_DONE
    ld cd, 0xFF30
    ld a, (cd)
    and a, 0x01
    ld 0x00FA, a
    cmp a, 0x00
    beq SC_INPUT_BEGIN_DONE
    ld a, (cd+0x01)
    ld 0x00FB, a
    ld a, (cd+0x02)
    ld 0x00FC, a
    ld a, 0x00
    ld 0xFF30, a
SC_INPUT_BEGIN_DONE:
    pop cd
    pop ab
    ret
SC_INPUT_END:
    push ab
    push cd
    ld cd, 0x00FA
    ld a, (cd)
    cmp a, 0x00
    beq SC_INPUT_END_DONE
    ld 0xFF30, a
    ld a, (cd+0x01)
    ld 0xFF31, a
    ld a, (cd+0x02)
    ld 0xFF32, a
    ld a, 0x00
    ld (cd), a
SC_INPUT_END_DONE:
    pop cd
    pop ab
    ret
; The frontend enters here only at an instruction boundary on Ctrl-C.
; PROMPT reclaims the stack and reveals the retained text page.
SC_BREAK:
    ld a, 0x04
    ld 0xFF10, a
    ld a, 0x00
    ld 0x00A6, a
    ld 0x00A8, a
    ld 0x00F4, a
    ld 0x00F9, a
    ld 0x00FA, a
    jmp PROMPT
