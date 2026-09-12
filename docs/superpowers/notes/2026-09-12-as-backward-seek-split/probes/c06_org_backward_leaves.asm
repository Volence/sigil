; shape c06_org_backward_leaves, written by gen.py
        cpu 68000
        org 0
        dc.l L_next
        dc.l L_after
        org $20
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org Start+2
        dc.w $EEEE
        org $10
L_next: dc.w $1234
        dc.l *
        cpu 68000
L_after: dc.w $5678
