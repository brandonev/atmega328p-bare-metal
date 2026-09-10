.section .vectors, "ax", @progbits
.global __vectors

__vectors:
    jmp reset                  ; 0: Reset
    jmp default_handler        ; 1: INT0
    jmp default_handler        ; 2: INT1
    jmp default_handler        ; 3: PCINT0
    jmp default_handler        ; 4: PCINT1
    jmp default_handler        ; 5: PCINT2
    jmp default_handler        ; 6: Watchdog
    jmp default_handler        ; 7: Timer2 Compare A
    jmp default_handler        ; 8: Timer2 Compare B
    jmp default_handler        ; 9: Timer2 Overflow
    jmp default_handler        ; 10: Timer1 Capture
    jmp timer1_compa_handler   ; 11: Timer1 Compare A


.section .text.reset, "ax", @progbits
.global reset

.extern rust_entry
.extern __data_load_start
.extern __data_start
.extern __data_end
.extern __bss_start
.extern __bss_end

reset:
    ; Rust's AVR calling convention expects r1 to contain zero.
    clr r1

    ; Clear SREG, including the global interrupt-enable bit.
    out 0x3F, r1

    ; Initialize the stack pointer to RAMEND: 0x08FF.
    ldi r16, 0x08
    out 0x3E, r16

    ldi r16, 0xFF
    out 0x3D, r16

    ; X (r27:r26) points to the destination .data section in SRAM.
    ldi r26, lo8(__data_start)
    ldi r27, hi8(__data_start)

    ; Z (r31:r30) points to the initial .data image in flash.
    ldi r30, lo8(__data_load_start)
    ldi r31, hi8(__data_load_start)

    ; r25:r24 holds the end address of .data in SRAM.
    ldi r24, lo8(__data_end)
    ldi r25, hi8(__data_end)

copy_data:
    ; Stop once X reaches __data_end.
    cp  r26, r24
    cpc r27, r25
    breq clear_bss_setup

    ; Copy one byte from flash to SRAM.
    lpm r0, Z+
    st  X+, r0
    rjmp copy_data


clear_bss_setup:
    ; X points to the beginning of .bss in SRAM.
    ldi r26, lo8(__bss_start)
    ldi r27, hi8(__bss_start)

    ; r25:r24 holds the end address of .bss.
    ldi r24, lo8(__bss_end)
    ldi r25, hi8(__bss_end)

clear_bss:
    ; Stop once X reaches __bss_end.
    cp  r26, r24
    cpc r27, r25
    breq data_initialized

    ; r1 contains zero.
    st  X+, r1
    rjmp clear_bss


data_initialized:
    call rust_entry

hang:
    rjmp hang


.section .text.default_handler, "ax", @progbits

default_handler:
    rjmp default_handler


.section .text.timer1.compa, "ax", @progbits
.global timer1_compa_handler
.extern timer1_compare_tick

timer1_compa_handler:
    ; Save r0 and the status register.
    push r0
    in   r0, 0x3F
    push r0

    ; Save r1, then restore Rust's zero-register invariant.
    push r1
    clr  r1

    ; Save call-clobbered registers.
    push r18
    push r19
    push r20
    push r21
    push r22
    push r23
    push r24
    push r25
    push r26
    push r27
    push r30
    push r31

    call timer1_compare_tick

    ; Restore registers in reverse order.
    pop r31
    pop r30
    pop r27
    pop r26
    pop r25
    pop r24
    pop r23
    pop r22
    pop r21
    pop r20
    pop r19
    pop r18

    pop r1

    ; Restore SREG and the original r0.
    pop r0
    out 0x3F, r0
    pop r0

    reti


.section .text.interrupts, "ax", @progbits
.global enable_interrupts

enable_interrupts:
    sei
    ret

.section .text.program_memory, "ax", @progbits
.global read_program_byte

read_program_byte:
    ; First u16 argument arrives in r25:r24.
    ; Z is r31:r30.
    movw r30, r24

    ; Read one byte from flash into the u8 return register.
    lpm r24, Z

    ret

.global save_and_disable_interrupts
.global restore_interrupts

save_and_disable_interrupts:
    ; A u8 return value uses r24.
    in  r24, 0x3F
    cli
    ret

restore_interrupts:
    ; The first u8 argument arrives in r24.
    out 0x3F, r24
    ret
