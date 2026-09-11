	cpu z80undoc
	org 0
	save
	cpu 68000
	restore
	ld a,ixl
	nop
	end
