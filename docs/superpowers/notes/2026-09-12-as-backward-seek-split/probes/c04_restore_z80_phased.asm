; shape c04_restore_z80_phased, written by gen.py
        cpu 68000
        org 0
        dc.l L_next
        dc.l L_after
        save
        cpu z80
        phase 8000h
        db 0AAh,0BBh,0CCh,0DDh
        org 8001h
        db 0EEh
        restore
        dephase
L_next: dc.w $1234
        dc.l *
        cpu 68000
L_after: dc.w $5678
