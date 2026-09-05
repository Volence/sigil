; The minimal form of the same mechanism: two range-refused immediates and NO
; accepted immediate above them. The word reads $0000 -- the initial state of
; the slot wcarry.asm shows being overwritten -- not a policy of answering zero.
	cpu 68000
	padding off
	phase 0
	move.w	#-32769,d0
	move.w	#65536,d0
	end
