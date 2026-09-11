	cpu 68000
	org 0
	move.w #$1234,d0
	save
	cpu z80undoc
	ld a,iyl
	restore
	move.w #$5678,d1
	nop
	end
