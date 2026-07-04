; Constructed Plan-5 harness root. Pulls ONLY the two Z80 phase blocks' includes
; plus the constant/struct/equate definitions they need. NO 68k code.
;
; Preamble (before region A's `phase 0`) MUST emit ZERO image bytes — only
; equ/struct/function definitions — or it opens a stray `sec0` (vma_base=None)
; that collides with region A's phase-0 sec0.
;
; `cpu 68000` here mirrors the reference build: main.asm is in 68000 context when
; it includes sound_constants.asm, so its `$`-prefixed hex literals lex correctly
; (under z80 `$` is the location counter, not a hex sigil). Region A's own
; `save/cpu z80/phase 0 … dephase/restore` switches to z80 and back for the
; driver body; region B does the same for the phase-08000h bank.
        cpu     68000
; `padding off` mirrors aeon main.asm:3 — the real build assembles in padding-off
; context, so DacSample and the other structs pack naively (ds.b/ds.w/ds.l do NOT
; even-round the running offset). Without this, DacSample sizes 10 vs the real 9.
        padding off
        include "sound_constants.asm"
; --- region A: resident driver, phase 0 ---
        include "engine/sound/z80_sound_driver.asm"
; --- region B: MT/SFX bank, phase 08000h ---
        save
        cpu     z80
        phase   08000h
        include "engine/sound/sound_tables_z80.asm"
        include "games/sonic4/data/sound/movingtrucks_pitchtable.asm"
        include "engine/sound/sfx_blob_win_tab.asm"
        include "engine/sound/seq_opcode_tab.asm"
        include "engine/sound/dac_sample_tab.asm"
        dephase
        restore
