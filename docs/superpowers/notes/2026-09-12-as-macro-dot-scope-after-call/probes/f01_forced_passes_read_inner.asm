; a11 shape with an unrelated forward reference forcing more passes; read `Inner.b`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	dc.w	Later
Base:	dc.w	$5555
mac	macro
Inner:	dc.w	$2222
	endm
	mac
.b	:=	2
	dc.w	Inner.b	; REF
Later:	dc.w	$9999
	dc.w	$4444
