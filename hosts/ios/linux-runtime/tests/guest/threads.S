.text
.global spawn_thread
spawn_thread:
    sub x2,x2,#16
    stp x0,x1,[x2]
    mov x1,x2
    mov x2,x4
    mov w0,#0x0f00
    movk w0,#0x013d,lsl #16
    mov x8,#220
    svc #0
    cbz x0,1f
    ret
1:
    ldp x9,x0,[sp],#16
    blr x9
    mov x0,#0
    mov x8,#93
    svc #0
    brk #0
