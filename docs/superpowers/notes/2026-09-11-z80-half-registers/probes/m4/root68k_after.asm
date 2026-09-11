	cpu 68000
	org 0
	save
	cpu z80undoc
	nop
	restore
	save
	cpu z80
	ld a,ixl
	restore
	nop
	end
