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
    ld cd, KW_REM
    call MATCH
    bne LONG_44
    jmp DONE
LONG_44:
    ld cd, KW_PRINT
    call MATCH
    bne LONG_47
    jmp PRINT
LONG_47:
    ld cd, KW_LET
    call MATCH
    bne LONG_50
    jmp ASSIGN
LONG_50:
    ld cd, KW_GOTO
    call MATCH
    bne LONG_53
    jmp GOTO
LONG_53:
    ld cd, KW_END
    call MATCH
    bne LONG_56
    jmp END_RUN
LONG_56:
    ld cd, KW_STOP
    call MATCH
    bne LONG_59
    jmp END_RUN
LONG_59:
    ld cd, KW_RUN
    call MATCH
    bne LONG_62
    jmp RUN
LONG_62:
    ld cd, KW_LIST
    call MATCH
    bne LONG_65
    jmp LIST
LONG_65:
    ld cd, KW_NEW
    call MATCH
    bne LONG_68
    jmp NEW
LONG_68:
    ld cd, KW_QUIT
    call MATCH
    bne LONG_71
    jmp QUIT
LONG_71:
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
    beq LONG_97
    jmp CLEAR_RECORD
LONG_97:
    ret
CLEAR_VARS:
    ld cd, 0x0300
    ld a, 0x00
CLEAR_VAR:
    ld (cd), a
    inc cd
    cmp cd, 0x0334
    beq LONG_106
    jmp CLEAR_VAR
LONG_106:
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
    bne LONG_117
    jmp EDIT_FOUND
LONG_117:
    cmp gh, 0x0000
    beq LONG_119
    jmp EDIT_NEXT
LONG_119:
    cmp ef, 0x0000
    beq LONG_121
    jmp EDIT_NEXT
LONG_121:
    ld ef, cd
EDIT_NEXT:
    clc
    add cd, 0x0080
    cmp cd, 0x9000
    beq LONG_127
    jmp EDIT_SCAN
LONG_127:
    cmp ef, 0x0000
    bne LONG_129
    jmp ERROR
LONG_129:
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
    beq LONG_143
    jmp EDIT_STORE
LONG_143:
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
    beq LONG_156
    jmp EDIT_COPY
LONG_156:
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
    bcs LONG_176
    jmp FIND_NO
LONG_176:
    bne LONG_177
    jmp FIND_NO
LONG_177:
    cmp cd, ef
    bcc LONG_179
    jmp FIND_NO
LONG_179:
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
    beq LONG_190
    jmp FIND_SCAN
LONG_190:
    pop gh
    ret
LIST:
    call EOL
    ld ab, 0x0000
LIST_NEXT:
    call FIND_NEXT
    cmp cd, 0x0000
    bne LONG_199
    jmp DONE
LONG_199:
    ld ab, ef
    push ab
    push cd
    call PRINT_NUM
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
    bne LONG_227
    jmp RUN_DONE
LONG_227:
    ld 0x0080, f
    ld 0x0081, e
    ld gh, cd
    inc gh
    inc gh
    call STATEMENT
    ld cd, 0x0082
    ld a, (cd)
    cmp a, 0x00
    beq LONG_237
    jmp RUN_NEXT
LONG_237:
RUN_DONE:
    ld a, 0x00
    ld 0x0082, a
    ret
GOTO:
    call EXPR
    push ab
    call EOL
    ld cd, 0x0082
    ld a, (cd)
    cmp a, 0x00
    bne LONG_249
    jmp ERROR
LONG_249:
    pop ab
    ; Require an exact target, then set current to target minus one.
    dec ab
    push ab
    call FIND_NEXT
    pop ab
    inc ab
    cmp ab, ef
    beq LONG_258
    jmp ERROR
LONG_258:
    dec ab
    ld 0x0080, b
    ld 0x0081, a
    ret
ASSIGN:
    call VARIABLE
    push cd
    call SPACE
    cmp a, 0x3D
    beq LONG_268
    jmp ERROR
LONG_268:
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
    bne LONG_281
    jmp NEWLINE
LONG_281:
PRINT_ITEM:
    cmp a, 0x22
    beq LONG_284
    jmp PRINT_VALUE
LONG_284:
    inc gh
PRINT_STRING:
    ld a, (gh)
    inc gh
    cmp a, 0x00
    bne LONG_290
    jmp ERROR
LONG_290:
    cmp a, 0x22
    bne LONG_292
    jmp PRINT_AFTER
LONG_292:
    call PUTCHAR
    jmp PRINT_STRING
PRINT_VALUE:
    call EXPR
    call PRINT_NUM
PRINT_AFTER:
    call SPACE
    cmp a, 0x3B
    bne LONG_301
    jmp PRINT_SEPARATOR
LONG_301:
    cmp a, 0x2C
    beq LONG_303
    jmp PRINT_END
LONG_303:
    ld a, 0x09
    call PUTCHAR
PRINT_SEPARATOR:
    inc gh
    call SPACE
    cmp a, 0x00
    bne LONG_310
    jmp DONE
LONG_310:
    jmp PRINT_ITEM
PRINT_END:
    call EOL
    jmp NEWLINE
EXPR:
    call SPACE
    cmp a, 0x30
    bcs LONG_318
    jmp EXPR_VAR
LONG_318:
    cmp a, 0x3A
    bcs LONG_320
    jmp NUMBER
LONG_320:
EXPR_VAR:
    call VARIABLE
    ld b, (cd)
    ld a, (cd+0x01)
    ret
VARIABLE:
    call SPACE
    cmp a, 0x41
    bcs LONG_329
    jmp ERROR
LONG_329:
    cmp a, 0x5B
    bcc LONG_331
    jmp ERROR
LONG_331:
    sec
    sub a, 0x41
    clc
    add a, a
    ld d, a
    ld c, 0x03
    inc gh
    ret
; Parse unsigned decimal to AB (stage 2: 0..32767).
NUMBER:
    push cd
    push ef
    ld ef, 0x0000
NUMBER_NEXT:
    call PEEK
    cmp a, 0x30
    bcs LONG_348
    jmp NUMBER_DONE
LONG_348:
    cmp a, 0x3A
    bcc LONG_350
    jmp NUMBER_DONE
LONG_350:
    sec
    sub a, 0x30
    ld b, a
    ld a, 0x00
    ld cd, ef
    clc
    add ef, ef
    clc
    add ef, ef
    clc
    add ef, cd
    clc
    add ef, ef
    bcc LONG_364
    jmp ERROR
LONG_364:
    clc
    add ef, ab
    bcc LONG_367
    jmp ERROR
LONG_367:
    cmp ef, 0x8000
    bcc LONG_369
    jmp ERROR
LONG_369:
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
    bcs LONG_403
    jmp DIGIT_END
LONG_403:
    sec
    sub ef, cd
    inc b
    jmp DIGIT_SUB
DIGIT_END:
    cmp b, 0x00
    beq LONG_410
    jmp DIGIT_EMIT
LONG_410:
    cmp gh, 0x0000
    bne LONG_412
    jmp DONE
LONG_412:
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
    bcs LONG_424
    jmp PEEK_DONE
LONG_424:
    cmp a, 0x7B
    bcc LONG_426
    jmp PEEK_DONE
LONG_426:
    sec
    sub a, 0x20
PEEK_DONE:
    ret
SPACE:
    call PEEK
    cmp a, 0x20
    bne LONG_434
    jmp SPACE_NEXT
LONG_434:
    cmp a, 0x09
    beq LONG_436
    jmp DONE
LONG_436:
SPACE_NEXT:
    inc gh
    jmp SPACE
EOL:
    call SPACE
    cmp a, 0x00
    beq LONG_443
    jmp ERROR
LONG_443:
    ret
MATCH:
    push gh
    push ef
MATCH_NEXT:
    ld e, (cd)
    cmp e, 0x00
    bne LONG_451
    jmp MATCH_BOUNDARY
LONG_451:
    call PEEK
    cmp a, e
    beq LONG_454
    jmp MATCH_FAIL
LONG_454:
    inc gh
    inc cd
    jmp MATCH_NEXT
MATCH_BOUNDARY:
    call PEEK
    cmp a, 0x41
    bcs LONG_461
    jmp MATCH_OK
LONG_461:
    cmp a, 0x5B
    bcs LONG_463
    jmp MATCH_FAIL
LONG_463:
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
    bne LONG_481
    jmp READLINE_NEXT
LONG_481:
    cmp a, 0x0A
    bne LONG_483
    jmp READLINE_END
LONG_483:
    cmp gh, 0x027A
    bcc LONG_485
    jmp ERROR
LONG_485:
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
    beq LONG_500
    jmp GETCHAR_READY
LONG_500:
    and b, 0x02
    cmp b, 0x00
    beq LONG_503
    jmp QUIT_EOF
LONG_503:
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
    bne LONG_521
    jmp DONE
LONG_521:
    call PUTCHAR
    inc cd
    jmp PUTS
ERROR:
    ld sp, 0xBFFF
    ld cd, ERROR_TEXT
    call PUTS
    jmp PROMPT
BANNER:
    data "RX-82 NATIVE BASIC", 0x0A, 0x00
PROMPT_TEXT:
    data "> ", 0x00
ERROR_TEXT:
    data "? ERROR", 0x0A, 0x00
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
