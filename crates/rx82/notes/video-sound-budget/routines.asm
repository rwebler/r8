; Capacity prototype, appended to native BASIC by build.py. Not shipped ROM.
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
    bne VIDEO_ILLEGAL
    pop ab
    pop cd
    ld 0xFF36, d
    ld 0xFF37, b
    ld a, 0x01
    ld 0xFF3A, a
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
    ld cd, 0xFF35
    ld a, (cd)
    shl a, 0x04
    ld b, a
    dec cd
    ld a, (cd)
    clc
    add a, b
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
