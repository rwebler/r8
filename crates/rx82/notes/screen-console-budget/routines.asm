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
    clc
    add b, h
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
    bne SC_EDIT_DONE
    ld a, (cd+0x06)
    cmp a, 0x02
    beq SC_EDIT_DONE
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
    bcs SC_EDIT_LOOP
    ld (ef), a
    inc ef
    call SC_OUTPUT
    jmp SC_EDIT_LOOP
SC_EDIT_BACKSPACE:
    cmp ef, 0xA400
    beq SC_EDIT_LOOP
    dec ef
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
