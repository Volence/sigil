; a body reads, BEFORE the include, a label the included file defines
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	dc.w	Linc	; REF
	include	"in4_inc.inc"
	endm
	mac
	mac
	dc.w	$4444
