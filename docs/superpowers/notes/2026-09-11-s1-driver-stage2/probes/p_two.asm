	cpu 68000
	padding off
Guess = 16
Guess2 = 12
	org 0
	dc.b 1,2,3,4,5,6,7,8
Drv:
	dc.b [16]$EE
	save
	!org 0
	cpu z80
	db 10h,11h,12h,13h,14h,15h,16h,17h,18h,19h
	restore
	padding off
	!org Drv+Guess
Drv2:
	dc.b [12]$DD
	save
	cpu z80
	!org 1300h
	db 20h,21h,22h,23h,24h,25h
	restore
	padding off
	!org Drv2+Guess2
	dc.b $AA,$BB
