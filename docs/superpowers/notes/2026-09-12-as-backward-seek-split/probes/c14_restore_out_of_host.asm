; shape c14_restore_out_of_host, written by gen.py
        cpu z80
        save
        cpu 68000
        org 0
        dc.l L_next
        dc.l L_after
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org Start+2
        dc.w $EEEE
        restore
        phase 8000h
L_next: db 12h,34h
        dw $
        dephase
        cpu 68000
L_after: dc.w $5678
