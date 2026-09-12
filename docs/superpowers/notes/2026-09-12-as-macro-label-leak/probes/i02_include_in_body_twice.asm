; the included label, macro invoked twice (collision?)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	include	"i02_inc.inc"
	endm
	mac
	mac
	dc.w	$3333
	dc.w	$4444
