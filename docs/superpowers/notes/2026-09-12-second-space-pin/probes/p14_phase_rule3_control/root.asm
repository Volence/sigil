	cpu 68000
Size1 equ $10
Size2 equ $10
	dc.l 0, 0
	org $100
	dc.w $4E71
	save
	!org $104
Mark:
	cpu z80
	phase 0
	db 1
	dephase
	restore
	padding off
	!org $110
	dc.w $4E71
