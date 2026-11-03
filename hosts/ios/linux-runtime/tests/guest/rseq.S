.text
.align 2
.global rseq_guest_increment
.global rseq_guest_start
.global rseq_guest_commit
.global rseq_guest_abort
rseq_guest_increment:
    str x2, [x0, #8]
rseq_guest_start:
    ldr w4, [x1]
    add w4, w4, #1
    str w4, [x1]
rseq_guest_commit:
    mov w0, #0
    ret
    .inst 0xd428bc00
rseq_guest_abort:
    mov w0, #1
    ret
