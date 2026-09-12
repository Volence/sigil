; shape c11_cpu_z80_phased, written by gen.py
        cpu 68000
        org 0
        dc.l L_next
        dc.l L_after
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org Start+2
        dc.w $EEEE
        cpu z80
        phase 8000h
L_next: db 12h,34h
        dw $
        dephase
        cpu 68000
L_after: dc.w $5678
