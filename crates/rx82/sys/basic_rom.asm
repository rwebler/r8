; RX-82 native BASIC. All text parsing, editing and execution runs on R8.
; RAM: 0080 current line, 0082 running, 0084 next line pointer.
; 0200 input buffer (128 bytes), 0300 variables (26 little-endian words).
; 1000..8FFF: 256 records of 128 bytes: line word, NUL-terminated text.
; A zero line number marks a free record. Stack grows down from BFFF.
    org 0xC000
BOOT:
    ld sp, 0xBFFF
    call NEW_PROGRAM
    ld cd, BANNER
    call PUTS
PROMPT:
    ld sp, 0xBFFF
    ld a, 0x00
    ld 0x0082, a
    ld cd, PROMPT_TEXT
    call PUTS
    call READLINE
    ld gh, 0x0200
    call SPACE
    cmp a, 0x00
    bne LONG_21
    jmp PROMPT
LONG_21:
    cmp a, 0x30
    bcs LONG_23
    jmp DIRECT
LONG_23:
    cmp a, 0x3A
    bcc LONG_25
    jmp DIRECT
LONG_25:
    call NUMBER
    cmp ab, 0x0000
    bne LONG_28
    jmp ERROR
LONG_28:
    push ab
    call SPACE
    pop ab
    call EDIT
    jmp PROMPT
DIRECT:
    call STATEMENT
    jmp PROMPT
; Command dispatch. MATCH advances GH only on success.
STATEMENT:
    call SPACE
    cmp a, 0x00
    bne LONG_41
    jmp DONE
LONG_41:
    ld cd, KW_FOR
    call MATCH
    bne LONG_44
    jmp FOR
LONG_44:
    ld cd, KW_NEXT
    call MATCH
    bne LONG_47
    jmp NEXT
LONG_47:
    ld cd, KW_IF
    call MATCH
    bne LONG_50
    jmp IF
LONG_50:
    ld cd, KW_GOSUB
    call MATCH
    bne LONG_53
    jmp GOSUB
LONG_53:
    ld cd, KW_RETURN
    call MATCH
    bne LONG_56
    jmp RETURN
LONG_56:
    ld cd, KW_INPUT
    call MATCH
    bne LONG_59
    jmp INPUT
LONG_59:
    ld cd, KW_REM
    call MATCH
    bne LONG_62
    jmp DONE
LONG_62:
    ld cd, KW_PRINT
    call MATCH
    bne LONG_65
    jmp PRINT
LONG_65:
    ld cd, KW_LET
    call MATCH
    bne LONG_68
    jmp ASSIGN
LONG_68:
    ld cd, KW_GOTO
    call MATCH
    bne LONG_71
    jmp GOTO
LONG_71:
    ld cd, KW_END
    call MATCH
    bne LONG_74
    jmp END_RUN
LONG_74:
    ld cd, KW_STOP
    call MATCH
    bne LONG_77
    jmp END_RUN
LONG_77:
    ld cd, KW_RUN
    call MATCH
    bne LONG_80
    jmp RUN
LONG_80:
    ld cd, KW_LIST
    call MATCH
    bne LONG_83
    jmp LIST
LONG_83:
    ld cd, KW_NEW
    call MATCH
    bne LONG_86
    jmp NEW
LONG_86:
    ld cd, KW_QUIT
    call MATCH
    bne LONG_89
    jmp QUIT
LONG_89:
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
    call EOL
    call NEW_PROGRAM
    ret
NEW_PROGRAM:
    ld cd, 0x1000
    ld a, 0x00
CLEAR_RECORD:
    ld (cd), a
    inc cd
    ld (cd), a
    clc
    add cd, 0x007F
    cmp cd, 0x9000
    beq LONG_115
    jmp CLEAR_RECORD
LONG_115:
    ret
CLEAR_VARS:
    ld cd, 0x0300
    ld a, 0x00
CLEAR_VAR:
    ld (cd), a
    inc cd
    cmp cd, 0x0334
    beq LONG_124
    jmp CLEAR_VAR
LONG_124:
    ret
; AB line number, GH body. Find matching slot or first empty slot.
EDIT:
    push ab
    ld cd, 0x1000
    ld ef, 0x0000
EDIT_SCAN:
    ld h, (cd)
    ld g, (cd+0x01)
    cmp gh, ab
    bne LONG_135
    jmp EDIT_FOUND
LONG_135:
    cmp gh, 0x0000
    beq LONG_137
    jmp EDIT_NEXT
LONG_137:
    cmp ef, 0x0000
    beq LONG_139
    jmp EDIT_NEXT
LONG_139:
    ld ef, cd
EDIT_NEXT:
    clc
    add cd, 0x0080
    cmp cd, 0x9000
    beq LONG_145
    jmp EDIT_SCAN
LONG_145:
    cmp ef, 0x0000
    bne LONG_147
    jmp ERROR
LONG_147:
    ld cd, ef
EDIT_FOUND:
    pop ab
    ; Reparse the line buffer to locate its body (scan used GH).
    push cd
    ld gh, 0x0200
    call NUMBER
    push ab
    call SPACE
    ld e, a
    pop ab
    pop cd
    cmp e, 0x00
    beq LONG_161
    jmp EDIT_STORE
LONG_161:
    ld ab, 0x0000
EDIT_STORE:
    ld (cd), b
    inc cd
    ld (cd), a
    inc cd
EDIT_COPY:
    ld a, (gh)
    ld (cd), a
    inc gh
    inc cd
    cmp a, 0x00
    beq LONG_174
    jmp EDIT_COPY
LONG_174:
    ret
; Find smallest stored line greater than AB. Returns EF line, CD record.
FIND_NEXT:
    push gh
    ld gh, 0x1000
    ld cd, 0x0000
    ld ef, 0xFFFF
FIND_SCAN:
    push ab
    ld b, (gh)
    ld a, (gh+0x01)
    ld 0x0088, b
    ld 0x0089, a
    pop ab
    push cd
    ld cd, 0x0088
    ld d, (cd)
    ld c, (gh+0x01)
    cmp cd, ab
    bcs LONG_194
    jmp FIND_NO
LONG_194:
    bne LONG_195
    jmp FIND_NO
LONG_195:
    cmp cd, ef
    bne LONG_197
    jmp FIND_CANDIDATE
LONG_197:
    bcc LONG_198
    jmp FIND_NO
LONG_198:
FIND_CANDIDATE:
    ld ef, cd
    pop cd
    ld cd, gh
    jmp FIND_ADVANCE
FIND_NO:
    pop cd
FIND_ADVANCE:
    clc
    add gh, 0x0080
    cmp gh, 0x9000
    beq LONG_210
    jmp FIND_SCAN
LONG_210:
    pop gh
    ret
LIST:
    call EOL
    ld ab, 0x0000
LIST_NEXT:
    call FIND_NEXT
    cmp cd, 0x0000
    bne LONG_219
    jmp DONE
LONG_219:
    ld ab, ef
    push ab
    push cd
    call PRINT_LINE
    ld a, 0x20
    call PUTCHAR
    pop cd
    inc cd
    inc cd
    call PUTS
    call NEWLINE
    pop ab
    jmp LIST_NEXT
RUN:
    call EOL
    call CLEAR_VARS
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
RUN_NEXT:
    ld cd, 0x0080
    ld b, (cd)
    ld a, (cd+0x01)
    call FIND_NEXT
    cmp cd, 0x0000
    bne LONG_253
    jmp RUN_DONE
LONG_253:
    ld 0x0080, f
    ld 0x0081, e
    ld gh, cd
    inc gh
    inc gh
    call STATEMENT
    ld cd, 0x0082
    ld a, (cd)
    cmp a, 0x00
    beq LONG_263
    jmp RUN_NEXT
LONG_263:
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
    bne LONG_278
    jmp ERROR
LONG_278:
    ; Require an exact target, then set current to target minus one.
    dec ab
    push ab
    call FIND_NEXT
    cmp cd, 0x0000
    bne LONG_284
    jmp ERROR
LONG_284:
    pop ab
    inc ab
    cmp ab, ef
    beq LONG_288
    jmp ERROR
LONG_288:
    ld cd, 0x00A4
    ld c, (cd)
    cmp c, 0x00
    bne LONG_292
    jmp GOTO_STORE
LONG_292:
    call PRUNE_LOOPS
GOTO_STORE:
    dec ab
    ld 0x0080, b
    ld 0x0081, a
    ret
ASSIGN:
    call VARIABLE
    push cd
    call SPACE
    cmp a, 0x3D
    beq LONG_304
    jmp ERROR
LONG_304:
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
    call SPACE
    cmp a, 0x00
    bne LONG_317
    jmp NEWLINE
LONG_317:
PRINT_ITEM:
    cmp a, 0x22
    beq LONG_320
    jmp PRINT_VALUE
LONG_320:
    inc gh
PRINT_STRING:
    ld a, (gh)
    inc gh
    cmp a, 0x00
    bne LONG_326
    jmp ERROR
LONG_326:
    cmp a, 0x22
    bne LONG_328
    jmp PRINT_AFTER
LONG_328:
    call PUTCHAR
    jmp PRINT_STRING
PRINT_VALUE:
    call EXPR
    call PRINT_NUM
PRINT_AFTER:
    call SPACE
    cmp a, 0x3B
    bne LONG_337
    jmp PRINT_SEPARATOR
LONG_337:
    cmp a, 0x2C
    beq LONG_339
    jmp PRINT_END
LONG_339:
    ld a, 0x09
    call PUTCHAR
PRINT_SEPARATOR:
    inc gh
    call SPACE
    cmp a, 0x00
    bne LONG_346
    jmp DONE
LONG_346:
    jmp PRINT_ITEM
PRINT_END:
    call EOL
    jmp NEWLINE
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
    bne LONG_363
    jmp EXPR_OPERATOR
LONG_363:
    cmp c, 0x2D
    beq LONG_365
    jmp EXPR_DONE
LONG_365:
EXPR_OPERATOR:
    inc gh
    push ab
    push cd
    call TERM
    ld ef, ab
    pop cd
    pop ab
    cmp c, 0x2B
    bne LONG_375
    jmp EXPR_ADD
LONG_375:
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
    bne LONG_395
    jmp TERM_OPERATOR
LONG_395:
    cmp c, 0x2F
    beq LONG_397
    jmp TERM_DONE
LONG_397:
TERM_OPERATOR:
    inc gh
    push ab
    push cd
    call VALUE
    ld ef, ab
    pop cd
    pop ab
    cmp c, 0x2A
    bne LONG_407
    jmp TERM_MULTIPLY
LONG_407:
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
    bcs LONG_420
    jmp ERROR
LONG_420:
    call SPACE
    cmp a, 0x28
    bne LONG_423
    jmp VALUE_PAREN
LONG_423:
    cmp a, 0x2D
    bne LONG_425
    jmp VALUE_NEG
LONG_425:
    cmp a, 0x2B
    bne LONG_427
    jmp VALUE_PLUS
LONG_427:
    cmp a, 0x30
    bcs LONG_429
    jmp VALUE_VAR
LONG_429:
    cmp a, 0x3A
    bcc LONG_431
    jmp VALUE_VAR
LONG_431:
    call NUMBER
    cmp ab, 0x8000
    bcc LONG_434
    jmp OVERFLOW
LONG_434:
    ret
VALUE_VAR:
    call VARIABLE
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
    beq LONG_450
    jmp ERROR
LONG_450:
    inc gh
    pop ab
    ret
VALUE_NEG:
    inc gh
    call SPACE
    cmp a, 0x30
    bcs LONG_458
    jmp VALUE_NEG_RECURSE
LONG_458:
    cmp a, 0x3A
    bcc LONG_460
    jmp VALUE_NEG_RECURSE
LONG_460:
    call NUMBER
    cmp ab, 0x8000
    bne LONG_463
    jmp NEGATE
LONG_463:
    bcc LONG_464
    jmp OVERFLOW
LONG_464:
    jmp NEGATE
VALUE_NEG_RECURSE:
    call VALUE
    cmp ab, 0x8000
    bne LONG_469
    jmp OVERFLOW
LONG_469:
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
    beq LONG_489
    jmp ADD_OK
LONG_489:
    ld d, a
    and d, 0x80
    cmp c, d
    beq LONG_493
    jmp OVERFLOW
LONG_493:
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
    bne LONG_506
    jmp ADD_OK
LONG_506:
    ld d, a
    and d, 0x80
    cmp c, d
    beq LONG_510
    jmp OVERFLOW
LONG_510:
    pop cd
    ret
; Normalize AB, EF to unsigned magnitudes and store result sign at 0090.
MAGNITUDES:
    ld c, 0x00
    cmp a, 0x80
    bcs LONG_517
    jmp MAG_RIGHT
LONG_517:
    call NEGATE
    inc c
MAG_RIGHT:
    cmp e, 0x80
    bcs LONG_522
    jmp MAG_DONE
LONG_522:
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
    bne LONG_537
    jmp MAG_POSITIVE
LONG_537:
    cmp ab, 0x8000
    bne LONG_539
    jmp NEGATE
LONG_539:
    bcc LONG_540
    jmp OVERFLOW
LONG_540:
    jmp NEGATE
MAG_POSITIVE:
    cmp ab, 0x8000
    bcc LONG_544
    jmp OVERFLOW
LONG_544:
    ret
MUL_SIGNED:
    push cd
    call MAGNITUDES
    ld cd, 0x0000
MUL_LOOP:
    cmp ab, 0x0000
    bne LONG_552
    jmp MUL_DONE
LONG_552:
    clc
    add cd, ef
    bcc LONG_555
    jmp OVERFLOW
LONG_555:
    dec ab
    jmp MUL_LOOP
MUL_DONE:
    ld ab, cd
    call MAG_RESULT
    pop cd
    ret
DIV_SIGNED:
    push cd
    cmp ef, 0x0000
    bne LONG_566
    jmp DIV_ZERO
LONG_566:
    call MAGNITUDES
    ld cd, 0x0000
DIV_LOOP:
    cmp ab, ef
    bcs LONG_571
    jmp DIV_DONE
LONG_571:
    sec
    sub ab, ef
    inc cd
    jmp DIV_LOOP
DIV_DONE:
    ld ab, cd
    call MAG_RESULT
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
    bcs LONG_594
    jmp ERROR
LONG_594:
    cmp a, 0x5B
    bcc LONG_596
    jmp ERROR
LONG_596:
    sec
    sub a, 0x41
    clc
    add a, a
    ld d, a
    ld c, 0x03
    inc gh
    ret
; Parse unsigned decimal to AB (0..65535); VALUE enforces signed limits.
NUMBER:
    push cd
    push ef
    ld ef, 0x0000
NUMBER_NEXT:
    call PEEK
    cmp a, 0x30
    bcs LONG_613
    jmp NUMBER_DONE
LONG_613:
    cmp a, 0x3A
    bcc LONG_615
    jmp NUMBER_DONE
LONG_615:
    sec
    sub a, 0x30
    ld b, a
    ld a, 0x00
    cmp ef, 0x1999
    bcs LONG_621
    jmp NUMBER_ACCUMULATE
LONG_621:
    beq LONG_622
    jmp OVERFLOW
LONG_622:
    cmp b, 0x06
    bcc LONG_624
    jmp OVERFLOW
LONG_624:
NUMBER_ACCUMULATE:
    ld cd, ef
    clc
    add ef, ef
    clc
    add ef, ef
    clc
    add ef, cd
    clc
    add ef, ef
    bcc LONG_635
    jmp ERROR
LONG_635:
    clc
    add ef, ab
    bcc LONG_638
    jmp ERROR
LONG_638:
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
    bcs LONG_651
    jmp PRINT_POSITIVE
LONG_651:
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
    clc
    add a, 0x30
    call PUTCHAR
    pop gh
    pop ef
    pop cd
    ret
PRINT_DIGIT:
    ld b, 0x00
DIGIT_SUB:
    cmp ef, cd
    bcs LONG_680
    jmp DIGIT_END
LONG_680:
    sec
    sub ef, cd
    inc b
    jmp DIGIT_SUB
DIGIT_END:
    cmp b, 0x00
    beq LONG_687
    jmp DIGIT_EMIT
LONG_687:
    cmp gh, 0x0000
    bne LONG_689
    jmp DONE
LONG_689:
DIGIT_EMIT:
    ld gh, 0x0001
    ld a, b
    clc
    add a, 0x30
    call PUTCHAR
    ret
; PEEK uppercases ASCII letters but does not modify source or other registers.
PEEK:
    ld a, (gh)
    cmp a, 0x61
    bcs LONG_701
    jmp PEEK_DONE
LONG_701:
    cmp a, 0x7B
    bcc LONG_703
    jmp PEEK_DONE
LONG_703:
    sec
    sub a, 0x20
PEEK_DONE:
    ret
SPACE:
    call PEEK
    cmp a, 0x20
    bne LONG_711
    jmp SPACE_NEXT
LONG_711:
    cmp a, 0x09
    beq LONG_713
    jmp DONE
LONG_713:
SPACE_NEXT:
    inc gh
    jmp SPACE
EOL:
    call SPACE
    cmp a, 0x00
    beq LONG_720
    jmp ERROR
LONG_720:
    ret
MATCH:
    push gh
    push ef
MATCH_NEXT:
    ld e, (cd)
    cmp e, 0x00
    bne LONG_728
    jmp MATCH_BOUNDARY
LONG_728:
    call PEEK
    cmp a, e
    beq LONG_731
    jmp MATCH_FAIL
LONG_731:
    inc gh
    inc cd
    jmp MATCH_NEXT
MATCH_BOUNDARY:
    call PEEK
    cmp a, 0x41
    bcs LONG_738
    jmp MATCH_OK
LONG_738:
    cmp a, 0x5B
    bcs LONG_740
    jmp MATCH_FAIL
LONG_740:
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
    ld gh, 0x0200
READLINE_NEXT:
    call GETCHAR
    cmp a, 0x0D
    bne LONG_758
    jmp READLINE_NEXT
LONG_758:
    cmp a, 0x0A
    bne LONG_760
    jmp READLINE_END
LONG_760:
    cmp gh, 0x027A
    bcc LONG_762
    jmp ERROR
LONG_762:
    ld (gh), a
    inc gh
    jmp READLINE_NEXT
READLINE_END:
    ld (gh), 0x00
    ret
GETCHAR:
    push cd
GETCHAR_WAIT:
    ld cd, 0xFF00
    ld a, (cd)
    ld b, a
    and a, 0x01
    cmp a, 0x00
    beq LONG_777
    jmp GETCHAR_READY
LONG_777:
    and b, 0x02
    cmp b, 0x00
    beq LONG_780
    jmp QUIT_EOF
LONG_780:
    jmp GETCHAR_WAIT
GETCHAR_READY:
    inc cd
    ld a, (cd)
    pop cd
    ret
QUIT_EOF:
    halt
PUTCHAR:
    ld 0xFF02, a
    ret
NEWLINE:
    ld a, 0x0A
    jmp PUTCHAR
PUTS:
    ld a, (cd)
    cmp a, 0x00
    bne LONG_798
    jmp DONE
LONG_798:
    call PUTCHAR
    inc cd
    jmp PUTS
ERROR:
    ld cd, ERROR_TEXT
REPORT_ERROR:
    ld sp, 0xBFFF
    call PUTS
    ld cd, 0x0082
    ld a, (cd)
    cmp a, 0x00
    bne LONG_810
    jmp ERROR_NEWLINE
LONG_810:
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
ERROR_TEXT:
    data "? ERROR", 0x00
KW_REM:
    data "REM", 0x00
KW_PRINT:
    data "PRINT", 0x00
KW_LET:
    data "LET", 0x00
KW_GOTO:
    data "GOTO", 0x00
KW_RUN:
    data "RUN", 0x00
KW_LIST:
    data "LIST", 0x00
KW_NEW:
    data "NEW", 0x00
KW_END:
    data "END", 0x00
KW_STOP:
    data "STOP", 0x00
KW_QUIT:
    data "QUIT", 0x00

REQUIRE_RUN:
    push ab
    push cd
    ld cd, 0x0082
    ld a, (cd)
    cmp a, 0x00
    bne LONG_855
    jmp ERROR
LONG_855:
    pop cd
    pop ab
    ret
; Signed comparison AB versus EF returns C=1 less, 2 equal, 4 greater.
COMPARE:
    cmp a, 0x80
    bcs LONG_862
    jmp COMP_LEFT_POS
LONG_862:
    cmp e, 0x80
    bcs LONG_864
    jmp COMP_LESS
LONG_864:
    jmp COMP_UNSIGNED
COMP_LEFT_POS:
    cmp e, 0x80
    bcc LONG_868
    jmp COMP_GREATER
LONG_868:
COMP_UNSIGNED:
    cmp ab, ef
    bcs LONG_871
    jmp COMP_LESS
LONG_871:
    bne LONG_872
    jmp COMP_EQUAL
LONG_872:
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
    call EXPR
    push ab
    call SPACE
    ld b, 0x02
    cmp a, 0x3D
    bne LONG_888
    jmp IF_OPERATOR_DONE
LONG_888:
    ld b, 0x01
    cmp a, 0x3C
    bne LONG_891
    jmp IF_LESS_OP
LONG_891:
    ld b, 0x04
    cmp a, 0x3E
    beq LONG_894
    jmp ERROR
LONG_894:
    inc gh
    call PEEK
    cmp a, 0x3D
    beq LONG_898
    jmp IF_RIGHT
LONG_898:
    ld b, 0x06
    jmp IF_OPERATOR_DONE
IF_LESS_OP:
    inc gh
    call PEEK
    cmp a, 0x3D
    bne LONG_905
    jmp IF_LE_OP
LONG_905:
    cmp a, 0x3E
    beq LONG_907
    jmp IF_RIGHT
LONG_907:
    ld b, 0x05
    jmp IF_OPERATOR_DONE
IF_LE_OP:
    ld b, 0x03
IF_OPERATOR_DONE:
    inc gh
IF_RIGHT:
    push b
    call EXPR
    ld ef, ab
    pop d
    pop ab
    call COMPARE
    push cd
    call SPACE
    ld cd, KW_THEN
    call MATCH
    beq LONG_925
    jmp ERROR
LONG_925:
    pop cd
    cmp c, d
    bne LONG_928
    jmp IF_TRUE
LONG_928:
    cmp d, 0x03
    beq LONG_930
    jmp IF_GE_TEST
LONG_930:
    cmp c, 0x04
    beq LONG_932
    jmp IF_TRUE
LONG_932:
    ret
IF_GE_TEST:
    cmp d, 0x06
    beq LONG_936
    jmp IF_NE_TEST
LONG_936:
    cmp c, 0x01
    beq LONG_938
    jmp IF_TRUE
LONG_938:
    ret
IF_NE_TEST:
    cmp d, 0x05
    beq LONG_942
    jmp DONE
LONG_942:
    cmp c, 0x02
    bne LONG_944
    jmp DONE
LONG_944:
IF_TRUE:
    call SPACE
    cmp a, 0x30
    bcs LONG_948
    jmp IF_STATEMENT
LONG_948:
    cmp a, 0x3A
    bcc LONG_950
    jmp IF_STATEMENT
LONG_950:
    jmp GOTO
IF_STATEMENT:
    ld cd, KW_FOR
    call MATCH
    bne LONG_955
    jmp ERROR
LONG_955:
    ld cd, KW_NEXT
    call MATCH
    bne LONG_958
    jmp ERROR
LONG_958:
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
    bcc LONG_971
    jmp ERROR
LONG_971:
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
    bne LONG_997
    jmp ERROR
LONG_997:
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
    ret
INPUT:
    call VARIABLE
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
    ret
; A lone unsigned literal can address any line. Expressions are signed 16-bit.
TARGET:
    call SPACE
    push gh
    cmp a, 0x30
    bcs LONG_1036
    jmp TARGET_EXPR
LONG_1036:
    cmp a, 0x3A
    bcc LONG_1038
    jmp TARGET_EXPR
LONG_1038:
    call NUMBER
    push ab
    call SPACE
    ld e, a
    pop ab
    cmp e, 0x00
    beq LONG_1045
    jmp TARGET_EXPR
LONG_1045:
    pop cd
    ret
TARGET_EXPR:
    pop gh
    jmp EXPR
KW_IF:
    data "IF", 0x00
KW_THEN:
    data "THEN", 0x00
KW_GOSUB:
    data "GOSUB", 0x00
KW_RETURN:
    data "RETURN", 0x00
KW_INPUT:
    data "INPUT", 0x00

; Loop frames at 0500..07FF, 16 bytes each:
; +0 FOR line, +2 NEXT line, +4 variable address, +6 limit, +8 step.
; 0094 points immediately past the active frames. Scratch 0098..00A5.
FOR:
    call REQUIRE_RUN
    call VARIABLE
    push cd
    call SPACE
    cmp a, 0x3D
    beq LONG_1071
    jmp ERROR
LONG_1071:
    inc gh
    call EXPR
    push ab
    call SPACE
    ld cd, KW_TO
    call MATCH
    beq LONG_1078
    jmp ERROR
LONG_1078:
    call EXPR
    push ab
    call SPACE
    ld cd, KW_STEP
    call MATCH
    beq LONG_1084
    jmp FOR_DEFAULT_STEP
LONG_1084:
    call EXPR
    jmp FOR_STEP_READY
FOR_DEFAULT_STEP:
    ld ab, 0x0001
FOR_STEP_READY:
    cmp ab, 0x0000
    bne LONG_1091
    jmp ERROR
LONG_1091:
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
    bne LONG_1113
    jmp FOR_FIND_END
LONG_1113:
    ld b, (gh+0x04)
    ld a, (gh+0x05)
    cmp ab, cd
    bne LONG_1117
    jmp ERROR
LONG_1117:
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
    bne LONG_1130
    jmp ERROR
LONG_1130:
    ld 0x00A0, f
    ld 0x00A1, e
    ld gh, cd
    inc gh
    inc gh
    call SPACE
    ld cd, KW_FOR
    call MATCH
    beq LONG_1139
    jmp FOR_SCAN_CLOSE
LONG_1139:
    inc (0x0096)
    jmp FOR_SCAN_ADVANCE
FOR_SCAN_CLOSE:
    ld cd, KW_NEXT
    call MATCH
    beq LONG_1145
    jmp FOR_SCAN_ADVANCE
LONG_1145:
    dec (0x0096)
    bne LONG_1147
    jmp FOR_MATCHED
LONG_1147:
FOR_SCAN_ADVANCE:
    ld cd, 0x00A0
    ld b, (cd)
    ld a, (cd+0x01)
    jmp FOR_SCAN_NEXT
FOR_MATCHED:
    call SPACE
    cmp a, 0x00
    bne LONG_1156
    jmp FOR_SET
LONG_1156:
    call VARIABLE
    push cd
    call EOL
    pop ef
    ld cd, 0x0098
    ld b, (cd)
    ld a, (cd+0x01)
    cmp ab, ef
    beq LONG_1165
    jmp ERROR
LONG_1165:
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
    bcc LONG_1181
    jmp FOR_NEGATIVE
LONG_1181:
    cmp b, 0x04
    bne LONG_1183
    jmp FOR_SKIP
LONG_1183:
    jmp FOR_PUSH
FOR_NEGATIVE:
    cmp b, 0x01
    bne LONG_1187
    jmp FOR_SKIP
LONG_1187:
FOR_PUSH:
    ld cd, 0x0094
    ld f, (cd)
    ld e, (cd+0x01)
    cmp ef, 0x0800
    bcc LONG_1193
    jmp ERROR
LONG_1193:
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
    ret
; CD = current call's loop base (0500 outside GOSUB).
LOOP_BASE:
    ld cd, 0x0092
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, 0x0500
    cmp ef, 0x0400
    bne LONG_1236
    jmp DONE
LONG_1236:
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
    bne LONG_1249
    jmp ERROR
LONG_1249:
    sec
    sub ab, 0x0010
    ld 0x00A2, b
    ld 0x00A3, a
    call SPACE
    cmp a, 0x00
    bne LONG_1256
    jmp NEXT_MATCH
LONG_1256:
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
    beq LONG_1268
    jmp ERROR
LONG_1268:
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
    beq LONG_1279
    jmp ERROR
LONG_1279:
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
    bcc LONG_1304
    jmp NEXT_NEGATIVE
LONG_1304:
    cmp d, 0x04
    bne LONG_1306
    jmp NEXT_POP
LONG_1306:
    jmp NEXT_REPEAT
NEXT_NEGATIVE:
    cmp d, 0x01
    bne LONG_1310
    jmp NEXT_POP
LONG_1310:
NEXT_REPEAT:
    ld b, (ef)
    ld a, (ef+0x01)
    ld 0x0080, b
    ld 0x0081, a
    ret
NEXT_POP:
    ld 0x0094, f
    ld 0x0095, e
    ret
KW_FOR:
    data "FOR", 0x00
KW_TO:
    data "TO", 0x00
KW_STEP:
    data "STEP", 0x00
KW_NEXT:
    data "NEXT", 0x00

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
    bne LONG_1340
    jmp PRUNE_DONE
LONG_1340:
    sec
    sub ef, 0x0010
    ld d, (ef)
    ld c, (ef+0x01)
    cmp ab, cd
    bcs LONG_1346
    jmp PRUNE_POP
LONG_1346:
    bne LONG_1347
    jmp PRUNE_POP
LONG_1347:
    ld d, (ef+0x02)
    ld c, (ef+0x03)
    cmp ab, cd
    bcs LONG_1351
    jmp PRUNE_DONE
LONG_1351:
    bne LONG_1352
    jmp PRUNE_DONE
LONG_1352:
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
