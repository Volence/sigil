	cpu z80
	org 0
	db firstbit(12)
	ld a,firstbit(12)
	ld hl,firstbit(12)
	db 0EEh
	end
