	cpu 68000
	padding off
	org 0
Y equ "a"
	move.w #Y,d0
	dc.b Y,$EE
	dc.b Y+1,$EE
	end
