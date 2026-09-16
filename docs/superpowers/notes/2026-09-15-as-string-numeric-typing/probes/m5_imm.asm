	cpu 68000
	padding off
	org 0
	move.b #"a",d0
	move.w #"a",d0
	move.w #"ab",d0
	move.l #"a",d0
	move.l #"ab",d0
	move.l #"abc",d0
	move.l #"abcd",d0
	move.w #"ab"+1,d0
	move.l #"abcd"+1,d0
	move.w #"a"+"b",d0
	move.l #"ab"+"cd",d0
	end
