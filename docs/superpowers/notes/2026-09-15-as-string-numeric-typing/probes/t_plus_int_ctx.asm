	cpu 68000
	padding off
	org 0
	move.w #"a"+"b",d0
	move.l #"ab"+"cd",d0
	move.w "a"+"b",d0
	move.l "abcd",d0
	move.w #"ab",d0
	move.b #"a",d0
	end
