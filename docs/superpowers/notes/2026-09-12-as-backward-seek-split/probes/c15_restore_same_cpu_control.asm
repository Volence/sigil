; shape c15_restore_same_cpu_control, written by gen.py
        cpu 68000
        org 0
        dc.l L_next
        dc.l L_after
        save
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org Start+2
        dc.w $EEEE
        restore
L_next: dc.w $1234
        dc.l *
        cpu 68000
L_after: dc.w $5678
