; body `Inner:`; after the call `.b := 2`; read `.b` (control: same scope both sides)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
Inner:	dc.w	$2222
	endm
	mac
.b	:=	2
	dc.w	.b	; REF
	dc.w	$4444
