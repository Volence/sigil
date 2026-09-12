; shape c18_next_reserves_over_the_tail, written by gen.py
        cpu 68000
        org 0
        dc.l L_next
        dc.l L_after
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org Start+2
        dc.w $EEEE
        cpu 68000
L_next: ds.b 4
        dc.w $1234
        dc.l *
        cpu 68000
L_after: dc.w $5678
