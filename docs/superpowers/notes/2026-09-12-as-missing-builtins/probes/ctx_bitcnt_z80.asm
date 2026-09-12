	cpu z80
	org 0
	db bitcnt(7)
	ld a,bitcnt(7)
	ld hl,bitcnt(7)
	db 0EEh
	end
