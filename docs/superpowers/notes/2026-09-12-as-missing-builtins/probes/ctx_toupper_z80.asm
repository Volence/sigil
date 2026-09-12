	cpu z80
	org 0
	db toupper(97)
	ld a,toupper(97)
	ld hl,toupper(97)
	db 0EEh
	end
