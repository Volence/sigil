; The same three undefined symbols as `three_undef_alone.asm`, with one
; unrelated error ABOVE them.
; asl: 1 pass, 1 error, exit 2, footer WITH the "not started" warning.
; Undefined symbols reported: ZERO.
	cpu	68000
	org	$1000
start:
	zzbogus	d0,d1
	move.w	#UND_ONE,d0
	move.w	#UND_TWO,d1
	move.w	#UND_THREE,d2
	rts
	end
