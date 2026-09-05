	cpu	68000
	padding	off
	org	0
; Q2: does the ENCLOSING CONSTRUCT matter for the INSIDE-the-expansion read?
; m17/m18 measured only the redefinition and the outside read. Each of `rept`,
; `irp` and `while` runs its body TWICE here at different addresses, so a wrong
; binding shows up as the same address printed twice.
	rept	2
Ra:
	dc.w	Ra
	endm
	irp	n,$11,$22
Ia:
	dc.b	n,0
	dc.w	Ia
	endm
Wc	set	0
	while	Wc<2
Wa:
	dc.w	Wa
Wc	set	Wc+1
	endm
	dc.w	$4444
