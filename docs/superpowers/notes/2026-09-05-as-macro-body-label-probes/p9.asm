	cpu	68000
	padding	off
	org	0
; Q1 (dotted locals) and the scope question. A `.local` label written inside a
; macro body: which scope does it qualify under, does it read back from inside
; the body, and is the QUALIFIED name reachable from outside? The macro runs
; twice under two DIFFERENT enclosing scopes, so "qualified under the file-level
; scope" and "qualified under something the expansion owns" are distinguishable.
mdot	macro
.dl:	dc.w	$1111
	dc.w	.dl
	endm
Sc1:
	mdot
Sc2:
	mdot
	dc.w	Sc1.dl
	dc.w	Sc2.dl
	dc.w	$4444
