; a label from a file included inside `rept 1`, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	rept	1
	include	"in6_inc.inc"
	endm
	dc.w	Linc	; REF
	dc.w	$4444
