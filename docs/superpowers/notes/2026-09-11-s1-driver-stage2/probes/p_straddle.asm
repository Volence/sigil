	cpu 68000
	padding off
Guess = $80
	org 0
	dc.b 1,2,3,4,5,6,7,8
Drv:
	save
	!org 0
	cpu z80
	binclude "p1.bin"
	restore
	padding off
	!org Drv+Guess
	dc.b $AA,$BB
