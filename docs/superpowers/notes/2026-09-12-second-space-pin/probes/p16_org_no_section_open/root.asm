	cpu 68000
Size1 equ $10
	dc.l 0, 0
	org $100
	dc.w $4E71
	cpu 68000
	save
	!org 0
DriverStart:
	cpu z80
	di
	ld a,1
	restore
	padding off
	!org $110
	dc.w $4E71
