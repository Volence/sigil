; an included file that defines and references its own label, from a body
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	include	"in2_inc.inc"
	endm
	mac
	mac
	dc.w	$4444
