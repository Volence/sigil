	cpu 68000
Size1 equ $10
Size2 equ $10
	dc.l 0, 0
	org $100
	dc.w $4E71
	save
	!org 0
D1:
	cpu z80
	di
	ld a,1
	restore
	padding off
	!org $110
	dc.w $4E71
	save
	!org $1300
D2:
	cpu z80
	db 7,8
	restore
	padding off
	!org $120
	dc.w $4E71
