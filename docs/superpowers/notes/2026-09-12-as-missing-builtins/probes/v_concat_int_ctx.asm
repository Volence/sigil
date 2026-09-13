	cpu 68000
	padding off
	org 0
X equ "a"+"b"
	dc.b X,$EE
	dc.w X
	move.w #X,d0
	move.w #"a"+"b",d0
	dc.w "a"+"b"
	end
