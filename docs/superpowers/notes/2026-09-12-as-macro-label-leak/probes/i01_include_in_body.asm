; a label in a file INCLUDED from a macro body, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	include	"i01_inc.inc"
	endm
	mac
	dc.w	Linc	; REF
	dc.w	$4444
