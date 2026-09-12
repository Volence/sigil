; shape c01_cpu_same, written by gen.py
        cpu 68000
        org 0
        dc.l L_next
        dc.l L_after
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org Start+2
        dc.w $EEEE
        cpu 68000
L_next: dc.w $1234
        dc.l *
        cpu 68000
L_after: dc.w $5678
