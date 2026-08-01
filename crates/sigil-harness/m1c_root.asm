; Bounded M1.C T10 harness root: the REAL games/sonic4/main.asm front-matter
; include tree + `org 0` + the 64-entry vector table, then STOP (main.asm
; continues into engine code needing generators). Faithful copy of main.asm's
; opening (verified) so a byte-match against s4.bin[0:256] proves the include
; tree parses and dc.l/symbol resolution work on real Aeon source.
;
; External CODE labels referenced by the vector table (EntryPoint, BusError, …)
; are seeded as `-D` defines / link stubs from s4.lst — see m1c_vector_table.rs.
; SYSTEM_STACK is NOT stubbed: it is a real `pub const` in engine/system/constants.emp,
; injected here as a guarded `-D` define via the harvest (see m1c_vector_table.rs).
; Include paths track the engine/game split (E1-E7): the front-matter now lives
; under engine/ + games/sonic4/config/; the vector table itself matches
; engine/system/vectors.asm verbatim.
    cpu 68000
    padding off
    supmode on

PAD_TO_POWER_OF_TWO     = 1

    ; Sound constants are authored in engine/sound/sound_constants.emp now; the
    ; residual AS reads none of its symbols (mirrors main.asm's front-matter).
    ; engine/macros.asm is DELETED (Parcel K4 — zero live consumers).
    ; parallax_macros.inc is retired (conv-g): the §4.6 authoring vocabulary is
    ; engine/level/parallax_dsl.emp; the config data is native.
    ; engine/sound/sound_bank.inc is DELETED (K4 inc-5 Stage 4b): the soundBankHead
    ; macro (the engine-table bank head) is native (games.sonic4.soundbankhead).
    ; games/sonic4/config/constants.asm (conv-f) and config/sound_ids.asm (F2) are
    ; retired: game constants live in constants.emp, the song/SFX ids in sound_ids.emp,
    ; the SFX-bank counts in sfx_bank.emp — all seeded here as harvested GUARDED -D
    ; defines (see m1c_vector_table.rs), mirroring the real build.
    include "games/sonic4/config/game.asm"
    ; engine/ram.asm (item #7b) AND games/sonic4/config/ram.asm (item #7c) are both
    ; retired: engine RAM is authored in engine/ram.emp, game RAM in
    ; games/sonic4/config/ram.emp (region-form `vars`). The vector table's one RAM
    ; reference (HBlank_Vector_Slot) and any game-RAM label resolve from the
    ; harvested engine+game RAM addresses seeded as -D defines (see
    ; m1c_vector_table.rs), mirroring the real build's Option-B bridge. The game RAM
    ; include is dropped exactly as main.asm's gameRamIncludes is now empty.
    include "engine/debug/debugger.asm"

    org 0

__BUDGET_VECTORS:
Vectors:
    dc.l    SYSTEM_STACK                    ; $00: Initial SSP
    dc.l    EntryPoint                      ; $04: Reset PC
    dc.l    BusError                        ; $08: Bus error
    dc.l    AddressError                    ; $0C: Address error
    dc.l    IllegalInstr                    ; $10: Illegal instruction
    dc.l    ZeroDivide                      ; $14: Division by zero
    dc.l    ChkInstr                        ; $18: CHK exception
    dc.l    TrapvInstr                      ; $1C: TRAPV
    dc.l    PrivilegeViol                   ; $20: Privilege violation
    dc.l    Trace                           ; $24: Trace
    dc.l    Line1010Emu                     ; $28: Line 1010
    dc.l    Line1111Emu                     ; $2C: Line 1111
    dc.l    ErrorExcept                     ; $30: Reserved
    dc.l    ErrorExcept                     ; $34: Reserved
    dc.l    ErrorExcept                     ; $38: Reserved
    dc.l    ErrorExcept                     ; $3C: Reserved
    dc.l    ErrorExcept                     ; $40: Reserved
    dc.l    ErrorExcept                     ; $44: Reserved
    dc.l    ErrorExcept                     ; $48: Reserved
    dc.l    ErrorExcept                     ; $4C: Reserved
    dc.l    ErrorExcept                     ; $50: Reserved
    dc.l    ErrorExcept                     ; $54: Reserved
    dc.l    ErrorExcept                     ; $58: Reserved
    dc.l    ErrorExcept                     ; $5C: Reserved
    dc.l    ErrorExcept                     ; $60: Spurious interrupt
    dc.l    NullInterrupt                   ; $64: IRQ1 (external)
    dc.l    NullInterrupt                   ; $68: IRQ2 (external)
    dc.l    NullInterrupt                   ; $6C: IRQ3
    dc.l    HBlank_Vector_Slot              ; $70: IRQ4 (HBlank) — RAM jmp-slot trampoline
    dc.l    NullInterrupt                   ; $74: IRQ5
    dc.l    VBlank_Handler                  ; $78: IRQ6 (VBlank)
    dc.l    NullInterrupt                   ; $7C: IRQ7 (NMI)
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $80-$8C: TRAP 0-3
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $90-$9C: TRAP 4-7
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $A0-$AC: TRAP 8-11
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $B0-$BC: TRAP 12-15
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $C0-$CC: Reserved
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $D0-$DC: Reserved
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $E0-$EC: Reserved
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $F0-$FC: Reserved
