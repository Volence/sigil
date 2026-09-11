	cpu 68000
	padding off
	org 0
n	equ 5
	move.w #"\{n}",d0
	dc.b $EE
	end
