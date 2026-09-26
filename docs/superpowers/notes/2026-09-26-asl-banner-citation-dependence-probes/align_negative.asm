; 2026-09-03-align-in-phase-contradiction.md, "What changed": asl truncates
; `n` to a 16-bit Word, so `align -256` acts as `align $FF00`. Probed at a PC
; where the two readings land differently: from $101, `align 256` gives $200
; and `align $FF00` gives $FF00.
	cpu 68000
	padding off
	org $100
	dc.b	$11
	align	-256
L:	dc.w	L
