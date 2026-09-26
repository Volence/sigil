; 2026-09-04-as-macro-default-params.md, "The rule, off the oracle": the
; declared default is substituted when no text is supplied; ARGCOUNT and an
; unshifted ALLARGS see no default. The note's own `ac` macro and calls.
	cpu 68000
	padding off
	org 0
ac	macro	p1,p2=DEF2,p3
	dc.b	ARGCOUNT
	dc.b	"<p1|p2|p3>"
	dc.b	"[ALLARGS]"
	endm
	ac
	ac	11
	ac	11,22
	ac	11,,33
	ac	p3=99
