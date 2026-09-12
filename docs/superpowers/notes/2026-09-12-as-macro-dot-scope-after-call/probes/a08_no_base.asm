; no caller label at all; after `.b := 2` read `Inner.b`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
Inner:	dc.w	$2222
	endm
	mac
.b	:=	2
	dc.w	Inner.b	; REF
	dc.w	$4444
