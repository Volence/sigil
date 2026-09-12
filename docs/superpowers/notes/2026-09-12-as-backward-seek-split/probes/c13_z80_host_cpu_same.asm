; shape c13_z80_host_cpu_same, written by gen.py
        cpu z80
        org 0
        dw L_next
        dw L_after
Start:  db 0AAh,0BBh,0CCh,0DDh
        org Start+1
        db 0EEh
        cpu z80
L_next: db 12h,34h
        dw $
        cpu z80
L_after: db 56h,78h
