	cpu	68000
	padding off
	org	0
; `abs(...)` in a PLAIN INTEGER context, with no float token anywhere on the
; line -- the shape `s1disasm/Macros.asm(353)` writes as
; `rept 1+(abs(first-last)/abs(step))`. Must exit 0.
	dc.l	ABS(-3)
	dc.l	1+(ABS(3-9)/ABS(2))	; the rept count, as a value
	rept	1+(ABS(3-9)/ABS(2))	; ... and as a rept count: 4 iterations
	dc.b	$AA
	endr
	dc.l	ABS(-3)+ABS(4)		; composes as an integer on both sides
	dc.b	ABS(-3)			; and fits a byte, so it is not a float
	end
