	cpu 68000
	padding off
Guess = $200
	org 0
	dc.b 1,2,3,4,5,6,7,8
Drv:
	save
	!org 0
	cpu z80
	db "The quick brown fox jumps over the lazy dog. The quick brown fox jumps again."
	db 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
	db 1,2,3,4,5,1,2,3,4,5,1,2,3,4,5,0F3h,0F3h,0F3h,31h,0FCh,1Fh,0DDh,21h,0,40h
	restore
	padding off
	!org Drv+Guess
	dc.b $AA,$BB
