; The control for `wrange.asm`: IDENTICAL except line 5 holds an accepted
; $1234 instead of the accepted -32768 ($8000). Lines 6-9, the range-refused
; ones, are byte-identical to wrange.asm. If the reference build ANSWERED a
; refused operand they would still read $8000; they read $1234, so what they
; carry is the last value asl computed, not a value for the line they are on.
	cpu 68000
	padding off
	phase 0
	move.w	#65535,d0
	move.w	#$1234,d0
	move.w	#-32769,d0
	move.w	#65536,d0
	move.w	#-65536,d0
	move.w	#$FFFFF700,d0
	end
