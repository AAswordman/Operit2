.text
.global namespace_spawn_thread
namespace_spawn_thread:
    sub x1,x1,#16
    str x0,[x1]
    mov x0,#0x0f00
    movk x0,#1,lsl #16
    mov x2,#0
    mov x3,#0
    mov x4,#0
    mov x8,#220
    svc #0
    cbz x0,1f
    ret
1:
    ldr x9,[sp],#16
    blr x9
    mov x0,#0
    mov x8,#93
    svc #0
    brk #0
