	cpu 68000
	padding off
	org 0
	move.w ($61).w,d0
	move.w "a",d0
	move.w "ab",d0
	move.l "abcd",d0
	move.w "ab"+1,d0
	move.w "a"+"b",d0
	lea "abcd",a0
	end
