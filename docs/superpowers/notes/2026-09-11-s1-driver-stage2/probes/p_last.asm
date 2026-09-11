	cpu 68000
	padding off
Guess = 16
	org 0
	dc.b 1,2,3,4,5,6,7,8
Drv:
	save
	!org 0
	cpu z80
	db 10h,11h,12h,13h,14h,15h,16h,17h,18h,19h
	restore
