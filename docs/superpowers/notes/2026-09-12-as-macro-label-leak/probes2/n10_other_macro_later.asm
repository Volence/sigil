; `-` defined in one macro's body, referenced from a later different macro
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mdef	macro
-	dc.w	$2222
	endm
mref	macro
	dc.w	-	; REF
	endm
	mdef
	mref
	dc.w	$4444
