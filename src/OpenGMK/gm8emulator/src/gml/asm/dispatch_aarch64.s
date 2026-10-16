/* OpenGMK AArch64 Pure Assembly Dispatch Loop
   Hand-crafted ARM64 assembly loop for GML instruction dispatching.
   Register pinning:
     x19: GML Stack Pointer
     x20: Jump Table Base Pointer
     x21: Instruction Pointer (PC)
*/
.text
.align 4
.global opengmk_dispatch_aarch64
.type opengmk_dispatch_aarch64, %function

opengmk_dispatch_aarch64:
    // Prologue: callee-saved registers
    stp x29, x30, [sp, #-48]!
    mov x29, sp
    stp x19, x20, [sp, #16]
    stp x21, x22, [sp, #32]

    // Args: x0 = stack_ptr, x1 = jump_table, x2 = insn_ptr, x3 = count
    mov x19, x0    // x19: GML Stack Pointer
    mov x20, x1    // x20: Opcode Jump Table Base Pointer
    mov x21, x2    // x21: PC (Instruction Pointer)
    mov x22, x3    // x22: Remaining instruction count

dispatch_loop:
    cbz x22, dispatch_exit
    // Fetch 32-bit opcode and advance PC
    ldr w4, [x21], #4
    // Direct indexed branch via jump table (eliminates branch misprediction)
    ldr x5, [x20, x4, lsl #3]
    blr x5
    sub x22, x22, #1
    b dispatch_loop

dispatch_exit:
    // Epilogue
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #48
    ret
