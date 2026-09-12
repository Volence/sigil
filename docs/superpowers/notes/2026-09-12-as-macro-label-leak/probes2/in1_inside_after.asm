; a label from a file included in a body, read later in the body
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	include	"in1_inc.inc"
	dc.w	Linc	; REF
	endm
	mac
	mac
	dc.w	$4444
