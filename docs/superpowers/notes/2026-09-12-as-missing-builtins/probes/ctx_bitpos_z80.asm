	cpu z80
	org 0
	db bitpos(8)
	ld a,bitpos(8)
	ld hl,bitpos(8)
	db 0EEh
	end
