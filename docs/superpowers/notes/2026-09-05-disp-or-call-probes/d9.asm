; What does the REFERENCE build put in an operand it declined to value?
;
; d8 answered `0000` for `#f(<reg>)` in all four cells; d6 answered `03C7` for
; the same shape. The only difference was ORDER — d6 evaluated `#konst(5)` on
; the line before. So the substitute is not a zero, it is THE LAST VALUE THIS
; BUILD COMPUTED, and the zero in d8 was only the initial state of that slot.
;
; Each declined `(a1)` line below is preceded by a successful call with a
; DIFFERENT, distinctive value. If the substitute is a stale carry-over, each
; `(a1)` line echoes the line above it and the three echoes differ from each
; other. If it is a zero, all three are `0000`.
	cpu 68000
one	function p,$0111
two	function p,$0222
three	function p,$0333
	org $1000
	move.w	#one(5),d0
	move.w	#one(a1),d0
	move.w	#two(5),d0
	move.w	#two(a1),d0
	move.w	#three(5),d0
	move.w	#three(a1),d0
