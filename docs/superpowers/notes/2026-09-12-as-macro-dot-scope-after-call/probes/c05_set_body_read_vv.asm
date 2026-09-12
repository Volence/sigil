; body `Vv set 5`; after `.b := 2` read `Vv.b`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
Vv	set	5
	endm
	mac
.b	:=	2
	dc.w	Vv.b	; REF
	dc.w	$4444
