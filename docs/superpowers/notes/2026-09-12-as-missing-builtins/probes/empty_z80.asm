	cpu z80
	org 0
	db abs()
	ld a,lastbit()
	db int()
	db 0EEh
	end
