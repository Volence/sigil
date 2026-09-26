; 2026-09-03-align-in-phase-contradiction.md, "What changed": `align 0` aborts
; asl with SIGFPE. The note does not say which build; a crash is the kind of
; answer builds are known to differ on (`dc.w a1` aborts on two of four).
	cpu 68000
	padding off
	org 0
	dc.b	$11
	align	0
	dc.b	$22
