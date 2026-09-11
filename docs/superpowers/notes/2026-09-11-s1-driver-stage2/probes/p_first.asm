	cpu 68000
	padding off
Guess = 16
Drv:
	save
	!org 0
	cpu z80
	db 10h,11h,12h,13h,14h,15h,16h,17h,18h,19h
	restore
	padding off
	!org $20
	dc.b $AA,$BB
