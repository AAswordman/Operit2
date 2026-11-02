.text
.global _start
_start:
    mov x0, sp
    bl guest_main
    mov x8, 93
    svc #0
    brk #0
