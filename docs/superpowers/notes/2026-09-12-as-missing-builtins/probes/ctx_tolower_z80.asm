	cpu z80
	org 0
	db tolower(65)
	ld a,tolower(65)
	ld hl,tolower(65)
	db 0EEh
	end
