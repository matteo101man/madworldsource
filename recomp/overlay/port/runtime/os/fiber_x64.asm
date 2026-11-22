; Minimal x64 (Windows ABI) stackful context switch for guest threads.
; Replaces Win32 fibers so guest stacks can live at fixed virtual addresses
; (required for the control-ready snapshot/restore). Saves the callee-saved
; integer registers, XMM6-15, MXCSR/x87 control word and the TEB stack bounds.
;
; struct mw_fiber_ctx { void* rsp; }  -- everything else is pushed on the stack.
;
; void mw_fiber_switch(mw_fiber_ctx* from, mw_fiber_ctx* to);
; void mw_fiber_prepare(mw_fiber_ctx* ctx, void* stack_top, void* stack_base,
;                       void (*entry)(void*), void* arg);
;   Builds an initial frame on [stack_base, stack_top) so that the first switch
;   into ctx calls entry(arg). entry must never return.

_TEXT SEGMENT
PUBLIC mw_fiber_switch, mw_fiber_prepare, mw_fiber_trampoline

; Frame layout pushed by mw_fiber_switch (offsets from saved rsp):
;  0: gs:[0x10] StackLimit     8: gs:[0x08] StackBase   16: gs:[0x1478] DeallocationStack
; 24: mxcsr(4) + fpu cw(2) + pad
; 32..191: xmm6..xmm15 (10 * 16)
; 192: r15 200: r14 208: r13 216: r12 224: rdi 232: rsi 240: rbp 248: rbx
; 256: return address

mw_fiber_switch PROC
    push rbx
    push rbp
    push rsi
    push rdi
    push r12
    push r13
    push r14
    push r15
    sub rsp, 192
    movdqu [rsp+32],  xmm6
    movdqu [rsp+48],  xmm7
    movdqu [rsp+64],  xmm8
    movdqu [rsp+80],  xmm9
    movdqu [rsp+96],  xmm10
    movdqu [rsp+112], xmm11
    movdqu [rsp+128], xmm12
    movdqu [rsp+144], xmm13
    movdqu [rsp+160], xmm14
    movdqu [rsp+176], xmm15
    stmxcsr dword ptr [rsp+24]
    fnstcw  word ptr  [rsp+28]
    mov rax, gs:[10h]
    mov [rsp+0], rax
    mov rax, gs:[08h]
    mov [rsp+8], rax
    mov rax, gs:[1478h]
    mov [rsp+16], rax
    mov [rcx], rsp            ; from->rsp = rsp
    mov rsp, [rdx]            ; rsp = to->rsp
    mov rax, [rsp+0]
    mov gs:[10h], rax
    mov rax, [rsp+8]
    mov gs:[08h], rax
    mov rax, [rsp+16]
    mov gs:[1478h], rax
    ldmxcsr dword ptr [rsp+24]
    fldcw   word ptr  [rsp+28]
    movdqu xmm6,  [rsp+32]
    movdqu xmm7,  [rsp+48]
    movdqu xmm8,  [rsp+64]
    movdqu xmm9,  [rsp+80]
    movdqu xmm10, [rsp+96]
    movdqu xmm11, [rsp+112]
    movdqu xmm12, [rsp+128]
    movdqu xmm13, [rsp+144]
    movdqu xmm14, [rsp+160]
    movdqu xmm15, [rsp+176]
    add rsp, 192
    pop r15
    pop r14
    pop r13
    pop r12
    pop rdi
    pop rsi
    pop rbp
    pop rbx
    ret
mw_fiber_switch ENDP

; Trampoline reached by the first switch: r12 = entry, r13 = arg (restored from
; the prepared frame). Keeps the stack 16-byte aligned at the call.
mw_fiber_trampoline PROC
    mov rcx, r13
    sub rsp, 32               ; shadow space
    call r12
    int 3                     ; entry must not return
mw_fiber_trampoline ENDP

; mw_fiber_prepare(rcx=ctx, rdx=stack_top, r8=stack_base, r9=entry, [rsp+40]=arg)
mw_fiber_prepare PROC
    mov rax, [rsp+40]         ; arg
    mov r10, rdx
    and r10, -16              ; align top
    sub r10, 256+8            ; frame (256 bytes) + return address slot
    ; Saved rsp is 8 mod 16, matching the switch frame. The trampoline
    ; receives a 16-aligned rsp and reserves shadow space before calling.
    ; zero the frame
    push rdi
    push rcx
    mov rdi, r10
    mov rcx, (256+8)/8
    xor r11, r11
@@:
    mov [rdi], r11
    add rdi, 8
    dec rcx
    jnz @B
    pop rcx
    pop rdi
    ; TEB stack bounds for this stack
    lea r11, [r8+10000h]
    mov [r10+0], r11          ; StackLimit above the inaccessible guard
    mov [r10+8], rdx          ; StackBase = top
    mov [r10+16], r8          ; DeallocationStack = base
    ; inherit the creator's control words
    stmxcsr dword ptr [r10+24]
    fnstcw word ptr [r10+28]
    ; r12 = entry, r13 = arg at their pop slots
    mov [r10+216], r9         ; r12 slot
    mov [r10+208], rax        ; r13 slot
    lea r11, mw_fiber_trampoline
    mov [r10+256], r11        ; return address -> trampoline
    mov [rcx], r10            ; ctx->rsp
    ret
mw_fiber_prepare ENDP

_TEXT ENDS
END
