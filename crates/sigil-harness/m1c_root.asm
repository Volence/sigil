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
    ; Review item 29 part 4 (the MDDBG strip): this harness targets the RELEASE
    ; (plain) vector table, and debugger.asm is now __DEBUG__-gated in the real
    ; game_root.asm (its MDDBG__* externs are unresolvable when error_handler is
    ; stripped). Mirror that gate — m1c_vector_table.rs assembles this root WITHOUT
    ; __DEBUG__, so the include is skipped (the Vectors label below is the guarded-
    ; EquSym carrier section the harvest needs).
    ifdef __DEBUG__
    include "engine/debug/debugger.asm"
    endif

    org 0

__BUDGET_VECTORS:
Vectors:
    ; RELEASE (plain) fault routing: every fault vector points at ReleaseFault
    ; (release_fault.emp — mask, red backdrop, freeze). The error_handler per-class
    ; stubs (BusError/…/ErrorExcept/ErrorTrap) exist only in the DEBUG shape now, so
    ; the plain s4.bin[0:256] this harness matches spells ReleaseFault throughout.
    dc.l    SYSTEM_STACK                    ; $00: Initial SSP
    dc.l    EntryPoint                      ; $04: Reset PC
    dc.l    ReleaseFault                    ; $08: Bus error
    dc.l    ReleaseFault                    ; $0C: Address error
    dc.l    ReleaseFault                    ; $10: Illegal instruction
    dc.l    ReleaseFault                    ; $14: Division by zero
    dc.l    ReleaseFault                    ; $18: CHK exception
    dc.l    ReleaseFault                    ; $1C: TRAPV
    dc.l    ReleaseFault                    ; $20: Privilege violation
    dc.l    ReleaseFault                    ; $24: Trace
    dc.l    ReleaseFault                    ; $28: Line 1010
    dc.l    ReleaseFault                    ; $2C: Line 1111
    dc.l    ReleaseFault                    ; $30: Reserved
    dc.l    ReleaseFault                    ; $34: Reserved
    dc.l    ReleaseFault                    ; $38: Reserved
    dc.l    ReleaseFault                    ; $3C: Reserved
    dc.l    ReleaseFault                    ; $40: Reserved
    dc.l    ReleaseFault                    ; $44: Reserved
    dc.l    ReleaseFault                    ; $48: Reserved
    dc.l    ReleaseFault                    ; $4C: Reserved
    dc.l    ReleaseFault                    ; $50: Reserved
    dc.l    ReleaseFault                    ; $54: Reserved
    dc.l    ReleaseFault                    ; $58: Reserved
    dc.l    ReleaseFault                    ; $5C: Reserved
    dc.l    ReleaseFault                    ; $60: Spurious interrupt
    dc.l    ReleaseFault                    ; $64: IRQ1 (unused level — halts loudly)
    dc.l    ReleaseFault                    ; $68: IRQ2 (external, controller TH — halts loudly)
    dc.l    ReleaseFault                    ; $6C: IRQ3 (unused level — halts loudly)
    dc.l    HBlank_Vector_Slot              ; $70: IRQ4 (HBlank) — RAM jmp-slot trampoline
    dc.l    ReleaseFault                    ; $74: IRQ5 (unused level — halts loudly)
    dc.l    VBlank_Handler                  ; $78: IRQ6 (VBlank)
    dc.l    ReleaseFault                    ; $7C: IRQ7/NMI (unused level — halts loudly)
    dc.l    ReleaseFault, ReleaseFault, ReleaseFault, ReleaseFault   ; $80-$8C: TRAP 0-3
    dc.l    ReleaseFault, ReleaseFault, ReleaseFault, ReleaseFault   ; $90-$9C: TRAP 4-7
    dc.l    ReleaseFault, ReleaseFault, ReleaseFault, ReleaseFault   ; $A0-$AC: TRAP 8-11
    dc.l    ReleaseFault, ReleaseFault, ReleaseFault, ReleaseFault   ; $B0-$BC: TRAP 12-15
    dc.l    ReleaseFault, ReleaseFault, ReleaseFault, ReleaseFault   ; $C0-$CC: Reserved
    dc.l    ReleaseFault, ReleaseFault, ReleaseFault, ReleaseFault   ; $D0-$DC: Reserved
    dc.l    ReleaseFault, ReleaseFault, ReleaseFault, ReleaseFault   ; $E0-$EC: Reserved
    dc.l    ReleaseFault, ReleaseFault, ReleaseFault, ReleaseFault   ; $F0-$FC: Reserved
