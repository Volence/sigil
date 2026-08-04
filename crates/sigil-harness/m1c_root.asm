; Bounded M1.C T10 harness root: the REAL games/sonic4/main.asm front-matter
; include tree + `org 0` + the 64-entry vector table, then STOP (main.asm
; continues into engine code needing generators). Faithful copy of main.asm's
; opening (verified) so a byte-match against s4.bin[0:256] proves the include
; tree parses and dc.l/symbol resolution work on real Aeon source.
;
; External CODE labels referenced by the vector table (EntryPoint, BusError, …) are
; seeded as `-D` defines / link stubs — see m1c_vector_table.rs.
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
    ; games/sonic4/config/game.asm (L1 P2) is retired: the game contract is `.emp`-
    ; native (engine.game_contract + games.sonic4.game). game_root.asm drops the
    ; include in lockstep; the vector table references no game-contract symbol.
    ; engine/ram.asm (item #7b) AND games/sonic4/config/ram.asm (item #7c) are both
    ; retired: engine RAM is authored in engine/ram.emp, game RAM in
    ; games/sonic4/config/ram.emp (region-form `vars`). The vector table's one RAM
    ; reference (HBlank_Vector_Slot) and any game-RAM label resolve from the
    ; harvested engine+game RAM addresses seeded as -D defines (see
    ; m1c_vector_table.rs), mirroring the real build's Option-B bridge. The game RAM
    ; include is dropped exactly as main.asm's gameRamIncludes is now empty.
    ;
    ; The real game_root.asm gates this include on __MDDBG__ (the crash-report axis:
    ; pushed when debug || crash_report, so both canonical shapes define it). This
    ; BOUNDED harness deliberately does NOT define it, so the include is skipped: it
    ; assembles only the front matter + the 64-entry table, and the debugger's MDDBG__*
    ; equ table would need the error_handler island this harness does not place. The
    ; vector table's 12 stub labels come in as -D / link stubs instead (see
    ; m1c_vector_table.rs), exactly as the other external CODE labels do. That gating
    ; difference is why this file mirrors the gate but leaves it unsatisfied.
    ifdef __MDDBG__
    include "engine/debug/debugger.asm"
    endif

    org 0

__BUDGET_VECTORS:
Vectors:
    ; RELEASE (plain) fault routing, as of the crash-report ruling (owner-ruled
    ; 2026-08-04): the error_handler island SHIPS in release, so the plain vector table
    ; names the same 12 per-class stubs the DEBUG one does — BusError/AddressError/…
    ; plus ErrorExcept (reserved/spurious/IRQ) and ErrorTrap (the 16 TRAPs). This
    ; mirrors engine/system/vectors.emp's `if DEBUG == 1 || CRASH_REPORT == 1` arm,
    ; which is the arm both canonical shapes take. ReleaseFault is the LEAN shape's
    ; handler and appears in no canonical listing, so it is gone from here.
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
    dc.l    ErrorExcept, ErrorExcept, ErrorExcept, ErrorExcept   ; $30-$3C: Reserved
    dc.l    ErrorExcept, ErrorExcept, ErrorExcept, ErrorExcept   ; $40-$4C: Reserved
    dc.l    ErrorExcept, ErrorExcept, ErrorExcept, ErrorExcept   ; $50-$5C: Reserved
    dc.l    ErrorExcept                     ; $60: Spurious interrupt
    dc.l    ErrorExcept                     ; $64: IRQ1 (unused level — halts loudly)
    dc.l    ErrorExcept                     ; $68: IRQ2 (external, controller TH — halts loudly)
    dc.l    ErrorExcept                     ; $6C: IRQ3 (unused level — halts loudly)
    dc.l    HBlank_Vector_Slot              ; $70: IRQ4 (HBlank) — RAM jmp-slot trampoline
    dc.l    ErrorExcept                     ; $74: IRQ5 (unused level — halts loudly)
    dc.l    VBlank_Handler                  ; $78: IRQ6 (VBlank)
    dc.l    ErrorExcept                     ; $7C: IRQ7/NMI (unused level — halts loudly)
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $80-$8C: TRAP 0-3
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $90-$9C: TRAP 4-7
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $A0-$AC: TRAP 8-11
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $B0-$BC: TRAP 12-15
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $C0-$CC: Reserved
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $D0-$DC: Reserved
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $E0-$EC: Reserved
    dc.l    ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap   ; $F0-$FC: Reserved
