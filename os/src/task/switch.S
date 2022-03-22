.altmacro
.macro SAVE_SN n
    sd s\n, (\n+2)*8(a1)
.endm
.macro LOAD_SN n
    ld s\n, (\n+2)*8(a0)
.endm
    .section .text

    # __switch(
    #     next_task_cx_ptr: *const TaskContext
    #     current_task_cx_ptr: *mut TaskContext,
    # )
    .globl __switch
    # __init(
    #     next_task_cx_ptr: *const TaskContext
    # )
    .globl __init

__switch:
    sd ra, 0(a1)
    sd sp, 8(a1)
    .set n, 0
    .rept 12
        SAVE_SN %n
        .set n, n + 1
    .endr

__init:
    .set n, 0
    .rept 12
        LOAD_SN %n
        .set n, n + 1
    .endr
    ld sp, 8(a0)
    ld ra, 0(a0)

    ret
