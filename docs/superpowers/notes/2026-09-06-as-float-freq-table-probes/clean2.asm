	cpu	68000
	padding off
	org	0
; A CLEAN run (exit 0) for the rows whose byte column is load-bearing. The
; type census in `types.asm` exits 2, and a failed run's byte column is not a
; source of values -- so `ABS(-3)` is re-asked here where the exit status
; permits reading it.
	dc.l	ABS(-3)		; INTEGER in, INTEGER out (no #1133)
	dc.l	ABS(-3)+1	; and it composes as an integer
; The corpus spells the name in LOWER case (`int(log(number))`), the probes
; above in UPPER. Both must reach the same builtin under `-U`.
	dc.l	INT(log(1000))
	dc.l	INT(Log(1000))
	dc.l	INT(lOg(1000))
; `log` of a float argument, and of an expression -- `hud_counter` is passed a
; macro parameter, so the argument arrives as tokens, not as a literal.
	dc.l	INT(LOG(10.0*100))
	end
