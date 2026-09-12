; `enum` member written in a file included from a body, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	include	"en6_inc.inc"
	endm
	mac
	dc.w	Eb	; REF
	dc.w	$4444
