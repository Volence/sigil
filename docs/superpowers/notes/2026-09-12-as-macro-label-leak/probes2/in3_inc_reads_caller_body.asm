; a body label read from a file included later in the same body
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
Lb:	dc.w	$2222
	include	"in3_inc.inc"
	endm
	mac
	mac
	dc.w	$4444
