; shape c16_trailing_ds_then_seek_to_it, written by gen.py
        cpu 68000
        org 0
        dc.l L_next
        dc.l L_after
Start:  dc.w $AAAA,$BBBB
        ds.b 4
        org Start+4
        cpu 68000
L_next: dc.w $1234
        dc.l *
        cpu 68000
L_after: dc.w $5678
