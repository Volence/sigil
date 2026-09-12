; body `Inner label *`; after `.b := 2` read `Inner.b`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
Inner	label	*
	endm
	mac
.b	:=	2
	dc.w	Inner.b	; REF
	dc.w	$4444
