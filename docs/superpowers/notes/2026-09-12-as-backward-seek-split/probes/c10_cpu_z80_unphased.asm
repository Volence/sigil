; shape c10_cpu_z80_unphased, written by gen.py
        cpu 68000
        org 0
        dc.l L_next
        dc.l L_after
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org Start+2
        dc.w $EEEE
        cpu z80
L_next: db 12h,34h
        cpu 68000
L_after: dc.w $5678
