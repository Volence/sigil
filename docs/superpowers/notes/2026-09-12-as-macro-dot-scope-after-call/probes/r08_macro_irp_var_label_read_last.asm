; body `irp v,Aa,Bb` / `v:`; after the call `.b := 2` read `Bb.b`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
	irp	v,Aa,Bb
v:	dc.w	$2222
	endm
	endm
	mac
.b	:=	2
	dc.w	Bb.b	; REF
	dc.w	$4444
