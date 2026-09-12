; `enum` member written in a file-level `rept 1` body, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	rept	1
	enum	Ea=5,Eb
	endm
	dc.w	Eb	; REF
	dc.w	$4444
