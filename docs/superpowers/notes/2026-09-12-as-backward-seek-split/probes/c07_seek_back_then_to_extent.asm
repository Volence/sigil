; shape c07_seek_back_then_to_extent, written by gen.py
        cpu 68000
        org 0
        dc.l L_next
        dc.l L_after
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org Start+2
        dc.w $EEEE
        org Start+8
        cpu 68000
L_next: dc.w $1234
        dc.l *
        cpu 68000
L_after: dc.w $5678
