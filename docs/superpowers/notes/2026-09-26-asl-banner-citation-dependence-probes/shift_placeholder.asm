; 2026-09-03-as-shift-macro-argument-walk.md: a parameter slot emptied BY A
; SHIFT keeps AS's two-byte \001\00N placeholder (strlen 2, "p"<>"" TRUE, the
; bytes leak into a string), while a never-bound slot is genuinely empty.
; Reconstructed from the note's prose; its own probe file was not committed.
	cpu 68000
	padding off
	org 0
ph	macro	p1,p2,p3,p4
	shift
	dc.b	strlen("p1"),strlen("p2"),strlen("p3"),strlen("p4")
	if "p2"<>""
	dc.b	$E2
	endif
	if "p3"<>""
	dc.b	$E3
	endif
	dc.b	"e[p1][p2][p3][p4]"
	endm
	ph	aa,bb
	ph	aa,bb,cc,dd
