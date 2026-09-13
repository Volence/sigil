	cpu 68000
	padding off
	org 0
f function x,"a"
Z equ f(1)
	move.w #Z,d0
	dc.b Z,$EE
	dc.b Z+1,$EE
	end
