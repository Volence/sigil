	cpu	68000
	padding	off
	org	0
; Q3: a macro-body label referenced from INSIDE the same expansion, BACKWARD.
; The macro is invoked TWICE and the label's address DIFFERS between the two
; expansions ($0 and $4), so a front end that bound the name globally and let
; the second expansion win would emit $0004 on BOTH lines. The two readings are
; distinguishable in the bytes, which is the whole point of invoking twice.
mi	macro
Li:	dc.w	$1111
	dc.w	Li
	endm
	mi
	mi
	dc.w	$4444
