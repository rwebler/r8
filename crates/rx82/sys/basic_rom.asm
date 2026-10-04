; RX-82 native BASIC. All text parsing, editing and execution runs on R8.
; Conditional jumps use an inverted short branch over an absolute JMP.
; Devices: FF00..02 console; FF10..14 raw file byte stream.
; 0092 subroutine stack pointer, 0094 loop stack pointer.
; 00A6..AC file transfer state (LOAD validates before replacing the program).
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
    bne LONG_25
    jmp PROMPT
LONG_25:
    cmp a, 0x30
    bcs LONG_27
    jmp DIRECT
LONG_27:
    cmp a, 0x3A
    bcc LONG_29
    jmp DIRECT
LONG_29:
    call NUMBER
    cmp ab, 0x0000
    bne LONG_32
    jmp INVALID_LINE_NUMBER
LONG_32:
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
    bne LONG_45
    jmp DONE
LONG_45:
    cmp a, 0x3F
    bne LONG_47
    jmp PRINT_SHORT
LONG_47:
    ld cd, KW_HELP
    call MATCH
    bne LONG_50
    jmp HELP
LONG_50:
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
    bne LONG_77
    jmp DONE
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
    bne LONG_89
    jmp END_RUN
LONG_89:
    ld cd, KW_STOP
    call MATCH
    bne LONG_92
    jmp END_RUN
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
    bne LONG_101
    jmp NEW
LONG_101:
    ld cd, KW_QUIT
    call MATCH
    bne LONG_104
    jmp QUIT
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
    ld cd, 0x1000
    ld a, 0x00
CLEAR_RECORD:
    ld (cd), a
    inc cd
    ld (cd), a
    clc
    add cd, 0x007F
    cmp cd, 0x9000
    beq LONG_132
    jmp CLEAR_RECORD
LONG_132:
    ret
CLEAR_VARS:
    ld cd, 0x0300
    ld a, 0x00
CLEAR_VAR:
    ld (cd), a
    inc cd
    cmp cd, 0x0334
    beq LONG_141
    jmp CLEAR_VAR
LONG_141:
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
    bne LONG_152
    jmp EDIT_FOUND
LONG_152:
    cmp gh, 0x0000
    beq LONG_154
    jmp EDIT_NEXT
LONG_154:
    cmp ef, 0x0000
    beq LONG_156
    jmp EDIT_NEXT
LONG_156:
    ld ef, cd
EDIT_NEXT:
    clc
    add cd, 0x0080
    cmp cd, 0x9000
    beq LONG_162
    jmp EDIT_SCAN
LONG_162:
    cmp ef, 0x0000
    bne LONG_164
    jmp PROGRAM_FULL
LONG_164:
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
    beq LONG_178
    jmp EDIT_STORE
LONG_178:
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
    beq LONG_191
    jmp EDIT_COPY
LONG_191:
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
    bcs LONG_211
    jmp FIND_NO
LONG_211:
    bne LONG_212
    jmp FIND_NO
LONG_212:
    cmp cd, ef
    bne LONG_214
    jmp FIND_CANDIDATE
LONG_214:
    bcc LONG_215
    jmp FIND_NO
LONG_215:
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
    beq LONG_227
    jmp FIND_SCAN
LONG_227:
    pop gh
    ret
LIST:
    call EOL
    ld ab, 0x0000
LIST_NEXT:
    call FIND_NEXT
    cmp cd, 0x0000
    bne LONG_236
    jmp DONE
LONG_236:
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
    bne LONG_270
    jmp RUN_DONE
LONG_270:
    ld 0x0080, f
    ld 0x0081, e
    ld gh, cd
    inc gh
    inc gh
    call STATEMENT
    ld cd, 0x0082
    ld a, (cd)
    cmp a, 0x00
    beq LONG_280
    jmp RUN_NEXT
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
    bne LONG_309
    jmp GOTO_STORE
LONG_309:
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
    call SPACE
    cmp a, 0x00
    bne LONG_334
    jmp NEWLINE
LONG_334:
PRINT_ITEM:
    cmp a, 0x22
    beq LONG_337
    jmp PRINT_VALUE
LONG_337:
    inc gh
PRINT_STRING:
    ld a, (gh)
    inc gh
    cmp a, 0x00
    bne LONG_343
    jmp UNTERMINATED_STRING
LONG_343:
    cmp a, 0x22
    bne LONG_345
    jmp PRINT_AFTER
LONG_345:
    call PUTCHAR
    jmp PRINT_STRING
PRINT_VALUE:
    call EXPR
    call PRINT_NUM
PRINT_AFTER:
    call SPACE
    cmp a, 0x3B
    bne LONG_354
    jmp PRINT_SEPARATOR
LONG_354:
    cmp a, 0x2C
    beq LONG_356
    jmp PRINT_END
LONG_356:
    ld a, 0x09
    call PUTCHAR
PRINT_SEPARATOR:
    inc gh
    call SPACE
    cmp a, 0x00
    bne LONG_363
    jmp DONE
LONG_363:
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
    bne LONG_380
    jmp EXPR_OPERATOR
LONG_380:
    cmp c, 0x2D
    beq LONG_382
    jmp EXPR_DONE
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
    bne LONG_392
    jmp EXPR_ADD
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
    bne LONG_412
    jmp TERM_OPERATOR
LONG_412:
    cmp c, 0x2F
    beq LONG_414
    jmp TERM_DONE
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
    bne LONG_424
    jmp TERM_MULTIPLY
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
    cmp a, 0x30
    bcs LONG_446
    jmp VALUE_VAR
LONG_446:
    cmp a, 0x3A
    bcc LONG_448
    jmp VALUE_VAR
LONG_448:
    call NUMBER
    cmp ab, 0x8000
    bcc LONG_451
    jmp OVERFLOW
LONG_451:
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
    beq LONG_467
    jmp EXPECTED_RPAREN
LONG_467:
    inc gh
    pop ab
    ret
VALUE_NEG:
    inc gh
    call SPACE
    cmp a, 0x30
    bcs LONG_475
    jmp VALUE_NEG_RECURSE
LONG_475:
    cmp a, 0x3A
    bcc LONG_477
    jmp VALUE_NEG_RECURSE
LONG_477:
    call NUMBER
    cmp ab, 0x8000
    bne LONG_480
    jmp NEGATE
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
    beq LONG_506
    jmp ADD_OK
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
    bne LONG_523
    jmp ADD_OK
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
    bcs LONG_534
    jmp MAG_RIGHT
LONG_534:
    call NEGATE
    inc c
MAG_RIGHT:
    cmp e, 0x80
    bcs LONG_539
    jmp MAG_DONE
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
    bne LONG_554
    jmp MAG_POSITIVE
LONG_554:
    cmp ab, 0x8000
    bne LONG_556
    jmp NEGATE
LONG_556:
    bcc LONG_557
    jmp OVERFLOW
LONG_557:
    jmp NEGATE
MAG_POSITIVE:
    cmp ab, 0x8000
    bcc LONG_561
    jmp OVERFLOW
LONG_561:
    ret
MUL_SIGNED:
    push cd
    call MAGNITUDES
    ld cd, 0x0000
MUL_LOOP:
    cmp ab, 0x0000
    bne LONG_569
    jmp MUL_DONE
LONG_569:
    clc
    add cd, ef
    bcc LONG_572
    jmp OVERFLOW
LONG_572:
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
    bne LONG_583
    jmp DIV_ZERO
LONG_583:
    call MAGNITUDES
    ld cd, 0x0000
DIV_LOOP:
    cmp ab, ef
    bcs LONG_588
    jmp DIV_DONE
LONG_588:
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
    bcs LONG_611
    jmp EXPECTED_VARIABLE
LONG_611:
    cmp a, 0x5B
    bcc LONG_613
    jmp EXPECTED_VARIABLE
LONG_613:
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
    bcs LONG_630
    jmp NUMBER_DONE
LONG_630:
    cmp a, 0x3A
    bcc LONG_632
    jmp NUMBER_DONE
LONG_632:
    sec
    sub a, 0x30
    ld b, a
    ld a, 0x00
    cmp ef, 0x1999
    bcs LONG_638
    jmp NUMBER_ACCUMULATE
LONG_638:
    beq LONG_639
    jmp OVERFLOW
LONG_639:
    cmp b, 0x06
    bcc LONG_641
    jmp OVERFLOW
LONG_641:
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
    bcs LONG_668
    jmp PRINT_POSITIVE
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
    bcs LONG_697
    jmp DIGIT_END
LONG_697:
    sec
    sub ef, cd
    inc b
    jmp DIGIT_SUB
DIGIT_END:
    cmp b, 0x00
    beq LONG_704
    jmp DIGIT_EMIT
LONG_704:
    cmp gh, 0x0000
    bne LONG_706
    jmp DONE
LONG_706:
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
    bcs LONG_718
    jmp PEEK_DONE
LONG_718:
    cmp a, 0x7B
    bcc LONG_720
    jmp PEEK_DONE
LONG_720:
    sec
    sub a, 0x20
PEEK_DONE:
    ret
SPACE:
    call PEEK
    cmp a, 0x20
    bne LONG_728
    jmp SPACE_NEXT
LONG_728:
    cmp a, 0x09
    beq LONG_730
    jmp DONE
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
    push gh
    push ef
MATCH_NEXT:
    ld e, (cd)
    cmp e, 0x00
    bne LONG_745
    jmp MATCH_BOUNDARY
LONG_745:
    call PEEK
    cmp a, e
    beq LONG_748
    jmp MATCH_FAIL
LONG_748:
    inc gh
    inc cd
    jmp MATCH_NEXT
MATCH_BOUNDARY:
    call PEEK
    cmp a, 0x41
    bcs LONG_755
    jmp MATCH_OK
LONG_755:
    cmp a, 0x5B
    bcs LONG_757
    jmp MATCH_FAIL
LONG_757:
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
    bne LONG_775
    jmp READLINE_NEXT
LONG_775:
    cmp a, 0x0A
    bne LONG_777
    jmp READLINE_END
LONG_777:
    cmp a, 0xFF
    bne LONG_779
    jmp READLINE_FILE_END
LONG_779:
    cmp a, 0x09
    bne LONG_781
    jmp READLINE_STORE
LONG_781:
    cmp a, 0x20
    bcs LONG_783
    jmp BAD_INPUT_CHARACTER
LONG_783:
READLINE_STORE:
    cmp gh, 0x027A
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
    ld (gh), 0x00
    ret
GETCHAR:
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
    beq LONG_810
    jmp GETCHAR_READY
LONG_810:
    and b, 0x02
    cmp b, 0x00
    beq LONG_813
    jmp QUIT_EOF
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
    bne LONG_828
    jmp PUTCHAR_CONSOLE
LONG_828:
    ld 0xFF13, a
    call FILE_CHECK
    jmp PUTCHAR_DONE
PUTCHAR_CONSOLE:
    ld 0xFF02, a
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
    jmp DONE
LONG_844:
    call PUTCHAR
    inc cd
    jmp PUTS
SYNTAX_ERROR:
    ld cd, SYNTAX_ERROR_TEXT
REPORT_ERROR:
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
    bne LONG_861
    jmp ERROR_NEWLINE
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
    bne LONG_906
    jmp PROGRAM_ONLY
LONG_906:
    pop cd
    pop ab
    ret
; Signed comparison AB versus EF returns C=1 less, 2 equal, 4 greater.
COMPARE:
    cmp a, 0x80
    bcs LONG_913
    jmp COMP_LEFT_POS
LONG_913:
    cmp e, 0x80
    bcs LONG_915
    jmp COMP_LESS
LONG_915:
    jmp COMP_UNSIGNED
COMP_LEFT_POS:
    cmp e, 0x80
    bcc LONG_919
    jmp COMP_GREATER
LONG_919:
COMP_UNSIGNED:
    cmp ab, ef
    bcs LONG_922
    jmp COMP_LESS
LONG_922:
    bne LONG_923
    jmp COMP_EQUAL
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
    call EXPR
    push ab
    call SPACE
    ld b, 0x02
    cmp a, 0x3D
    bne LONG_939
    jmp IF_OPERATOR_DONE
LONG_939:
    ld b, 0x01
    cmp a, 0x3C
    bne LONG_942
    jmp IF_LESS_OP
LONG_942:
    ld b, 0x04
    cmp a, 0x3E
    beq LONG_945
    jmp EXPECTED_COMPARISON
LONG_945:
    inc gh
    call PEEK
    cmp a, 0x3D
    beq LONG_949
    jmp IF_RIGHT
LONG_949:
    ld b, 0x06
    jmp IF_OPERATOR_DONE
IF_LESS_OP:
    inc gh
    call PEEK
    cmp a, 0x3D
    bne LONG_956
    jmp IF_LE_OP
LONG_956:
    cmp a, 0x3E
    beq LONG_958
    jmp IF_RIGHT
LONG_958:
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
    beq LONG_976
    jmp EXPECTED_THEN
LONG_976:
    pop cd
    cmp c, d
    bne LONG_979
    jmp IF_TRUE
LONG_979:
    cmp d, 0x03
    beq LONG_981
    jmp IF_GE_TEST
LONG_981:
    cmp c, 0x04
    beq LONG_983
    jmp IF_TRUE
LONG_983:
    ret
IF_GE_TEST:
    cmp d, 0x06
    beq LONG_987
    jmp IF_NE_TEST
LONG_987:
    cmp c, 0x01
    beq LONG_989
    jmp IF_TRUE
LONG_989:
    ret
IF_NE_TEST:
    cmp d, 0x05
    beq LONG_993
    jmp DONE
LONG_993:
    cmp c, 0x02
    bne LONG_995
    jmp DONE
LONG_995:
IF_TRUE:
    call SPACE
    cmp a, 0x30
    bcs LONG_999
    jmp IF_STATEMENT
LONG_999:
    cmp a, 0x3A
    bcc LONG_1001
    jmp IF_STATEMENT
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
    bcs LONG_1087
    jmp TARGET_EXPR
LONG_1087:
    cmp a, 0x3A
    bcc LONG_1089
    jmp TARGET_EXPR
LONG_1089:
    call NUMBER
    push ab
    call SPACE
    ld e, a
    pop ab
    cmp e, 0x00
    beq LONG_1096
    jmp TARGET_EXPR
LONG_1096:
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
    beq LONG_1135
    jmp FOR_DEFAULT_STEP
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
    bne LONG_1164
    jmp FOR_FIND_END
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
    inc gh
    inc gh
    call SPACE
    ld cd, KW_FOR
    call MATCH
    beq LONG_1190
    jmp FOR_SCAN_CLOSE
LONG_1190:
    inc (0x0096)
    jmp FOR_SCAN_ADVANCE
FOR_SCAN_CLOSE:
    ld cd, KW_NEXT
    call MATCH
    beq LONG_1196
    jmp FOR_SCAN_ADVANCE
LONG_1196:
    dec (0x0096)
    bne LONG_1198
    jmp FOR_MATCHED
LONG_1198:
FOR_SCAN_ADVANCE:
    ld cd, 0x00A0
    ld b, (cd)
    ld a, (cd+0x01)
    jmp FOR_SCAN_NEXT
FOR_MATCHED:
    call SPACE
    cmp a, 0x00
    bne LONG_1207
    jmp FOR_SET
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
    bcc LONG_1232
    jmp FOR_NEGATIVE
LONG_1232:
    cmp b, 0x04
    bne LONG_1234
    jmp FOR_SKIP
LONG_1234:
    jmp FOR_PUSH
FOR_NEGATIVE:
    cmp b, 0x01
    bne LONG_1238
    jmp FOR_SKIP
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
    ret
; CD = current call's loop base (0500 outside GOSUB).
LOOP_BASE:
    ld cd, 0x0092
    ld f, (cd)
    ld e, (cd+0x01)
    ld cd, 0x0500
    cmp ef, 0x0400
    bne LONG_1287
    jmp DONE
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
    bne LONG_1307
    jmp NEXT_MATCH
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
    bcc LONG_1355
    jmp NEXT_NEGATIVE
LONG_1355:
    cmp d, 0x04
    bne LONG_1357
    jmp NEXT_POP
LONG_1357:
    jmp NEXT_REPEAT
NEXT_NEGATIVE:
    cmp d, 0x01
    bne LONG_1361
    jmp NEXT_POP
LONG_1361:
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
    bne LONG_1391
    jmp PRUNE_DONE
LONG_1391:
    sec
    sub ef, 0x0010
    ld d, (ef)
    ld c, (ef+0x01)
    cmp ab, cd
    bcs LONG_1397
    jmp PRUNE_POP
LONG_1397:
    bne LONG_1398
    jmp PRUNE_POP
LONG_1398:
    ld d, (ef+0x02)
    ld c, (ef+0x03)
    cmp ab, cd
    bcs LONG_1402
    jmp PRUNE_DONE
LONG_1402:
    bne LONG_1403
    jmp PRUNE_DONE
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
    call SPACE
    cmp a, 0x22
    beq LONG_1431
    jmp EXPECTED_FILENAME
LONG_1431:
    inc gh
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
    bne LONG_1441
    jmp FILENAME_END
LONG_1441:
    ld 0xFF12, a
    call FILE_CHECK
    jmp FILENAME_CHAR
FILENAME_END:
    call EOL
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
    beq LONG_1489
    jmp LOAD_EOF
LONG_1489:
    call READLINE
    ld gh, 0x0200
    call SPACE
    cmp a, 0x00
    bne LONG_1494
    jmp LOAD_LINE
LONG_1494:
    cmp a, 0x30
    bcs LONG_1496
    jmp INVALID_LINE_NUMBER
LONG_1496:
    cmp a, 0x3A
    bcc LONG_1498
    jmp INVALID_LINE_NUMBER
LONG_1498:
    call NUMBER
    cmp ab, 0x0000
    bne LONG_1501
    jmp INVALID_LINE_NUMBER
LONG_1501:
    push ab
    call SPACE
    pop ab
    ld cd, 0x00AC
    ld c, (cd)
    cmp c, 0x00
    beq LONG_1508
    jmp LOAD_INSTALL
LONG_1508:
    ld cd, 0x00AA
    ld b, (cd)
    ld a, (cd+0x01)
    inc ab
    cmp ab, 0x0101
    bcc LONG_1514
    jmp PROGRAM_FULL
LONG_1514:
    ld (cd), b
    ld (cd+0x01), a
    jmp LOAD_LINE
LOAD_INSTALL:
    call EDIT
    jmp LOAD_LINE
LOAD_EOF:
    ld cd, 0x00AC
    ld a, (cd)
    cmp a, 0x00
    beq LONG_1525
    jmp LOAD_DONE
LONG_1525:
    ; Rewind the immutable byte snapshot, then install. Validation has not
    ; touched program records or variables. The second pass cannot exceed RAM.
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
    beq LONG_1551
    jmp GETFILE_EOF
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
KW_SAVE:
    data "SAVE", 0x00
KW_LOAD:
    data "LOAD", 0x00

PRINT_SHORT:
    inc gh
    jmp PRINT
HELP:
    call EOL
    ld cd, HELP_TEXT
    jmp PUTS
KW_HELP:
    data "HELP", 0x00
HELP_TEXT:
    data "RX-82 native ROM: numbered lines; LIST RUN NEW SAVE LOAD QUIT", 0x0A
    data "LET PRINT INPUT IF THEN GOTO GOSUB RETURN FOR TO STEP NEXT END", 0x0A
    data "A-Z variables, signed 16-bit integers, + - * / and parentheses", 0x0A, 0x00

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
