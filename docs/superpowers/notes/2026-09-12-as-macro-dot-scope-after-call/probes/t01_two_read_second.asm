; body `In1:` `In2:`; after `.b := 2` read `In2.b`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
In1:	dc.w	$2222
In2:	dc.w	$2223
	endm
	mac
.b	:=	2
	dc.w	In2.b	; REF
	dc.w	$4444
