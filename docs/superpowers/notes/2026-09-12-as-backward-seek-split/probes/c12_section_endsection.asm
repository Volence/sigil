; shape c12_section_endsection, written by gen.py
        cpu 68000
        org 0
        dc.l L_next
        dc.l L_after
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org Start+2
        dc.w $EEEE
        section blk
        public L_next
L_next: dc.w $1234
        dc.l *
        endsection blk
        cpu 68000
L_after: dc.w $5678
