	cpu 68000
	padding off
Guess = 16
	org 0
	dc.b 1,2,3,4,5,6,7,8
Drv:
	save
	!org 0
	cpu z80
	db 10h,11h,12h
	!org 8
	db 20h,21h
	restore
	padding off
	!org Drv+Guess
	dc.b $AA,$BB
