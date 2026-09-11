	cpu z80
	org 0
ixl equ 5
	save
	cpu z80undoc
	restore
	ld a,ixl
	nop
	end
