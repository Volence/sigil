; macro called twice; after `.b := 2` read `Inner.b`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
Inner:	dc.w	$2222
	endm
	mac
	mac
.b	:=	2
	dc.w	Inner.b	; REF
	dc.w	$4444
