; shape c20_org_back_to_the_cursor_after_other_code, written by gen.py
        cpu 68000
        org 0
        dc.l L_next
        dc.l L_after
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org Start+2
        dc.w $EEEE
        cpu 68000
        org $20
        dc.w $9999
        org Start+4
L_next: dc.w $1234
        dc.l *
        cpu 68000
L_after: dc.w $5678
