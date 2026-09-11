	cpu z80
	org 0
	db lastbit(5)
	ld a,lastbit(9)
	db 0EEh
	end
