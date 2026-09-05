	cpu	68000
	padding	off
	org	0
; --- phase/dephase around a redeclaration ---
Ap:
	dc.w	$1111
	phase	$1000
Bp:
	dc.w	$2222
	dephase
; the same name again, now at a different real address AND a different logical
; one, so a refusal here cannot be read as "the value did not move"
Bp:
	dc.w	$2222
	dc.w	$4444
