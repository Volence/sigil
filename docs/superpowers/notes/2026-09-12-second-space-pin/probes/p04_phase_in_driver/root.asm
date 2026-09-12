	cpu 68000
Size1 equ $10
Size2 equ $10
	dc.l 0, 0
	org $100
	dc.w $4E71
	save
	!org 0
	cpu z80
	di
	org 40h
L40:
	phase 1000h
	db 5
	dephase
	restore
	padding off
	!org $110
	dc.w $4E71
