	cpu 68000
Size1 equ $10
Size2 equ $10
	dc.l 0, 0
	org $100
	dc.w $4E71
	save
	!org 0
DriverStart:
	cpu z80
	db 1,2,3,4
	org 2
	db 9
	cpu z80
Next:
	db 7
	restore
	padding off
	!org $110
	dc.w $4E71
