; body `irp` / `Inner:`; after the call `.b := 2` read `Base.b`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
	irp	v,1
Inner:	dc.w	$2222
	endm
	endm
	mac
.b	:=	2
	dc.w	Base.b	; REF
	dc.w	$4444
