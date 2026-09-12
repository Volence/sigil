; label from mdef read inside an EARLIER expansion of a different macro
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mdef	macro
Lp:	dc.w	$2222
	endm
mref	macro
	dc.w	Lp	; REF
	endm
	mref
	mdef
	dc.w	$4444
