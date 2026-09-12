; a column-1 `-` in a file included from a macro body, `-` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	include	"i03_inc.inc"
	endm
	mac
	dc.w	-	; REF
	dc.w	$4444
