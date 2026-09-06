	cpu	68000
	padding off
	org	0
; UNARY PLUS. `s1disasm`'s `range` macro is invoked as `range $21,$2F,+1`, so
; `abs(step)` arrives as `abs(+1)`. sigil's typed evaluator has an arm for
; unary minus and none for unary plus, which is why four of the eight `range`
; sites still refused after `abs` was wired. Must exit 0.
	dc.l	ABS(+1)
	dc.l	+5
	dc.l	1+(ABS($21-$2F)/ABS(+1))	; the real rept count, ascending
	dc.l	1+(ABS($2F-$21)/ABS(-1))	; ... and descending
	dc.l	INT(+3.7)
; `INT(+3.7+ +1)` is NOT here on purpose: asl answers `error #1110: wrong
; number of operands / expected 2 arguments but got 1` for it, so a second `+`
; inside the parentheses is read as an argument separator rather than as a sign.
; The line was removed rather than kept as a refusal row, because keeping it
; made this whole file exit 2 and a failed run's byte column is not a source of
; values for the lines that DID assemble.
	end
