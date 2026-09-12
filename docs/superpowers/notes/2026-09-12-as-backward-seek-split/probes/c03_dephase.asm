; shape c03_dephase, written by gen.py
        cpu 68000
        org 0
        dc.l L_next
        dc.l L_after
        phase $8000
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org $8002
        dc.w $EEEE
        dephase
L_next: dc.w $1234
        dc.l *
        cpu 68000
L_after: dc.w $5678
