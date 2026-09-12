; body `Inner:` then `.v := 5`; read `Base.v` after the call
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
Inner:	dc.w	$2222
.v	:=	5
	endm
	mac
	dc.w	Base.v	; REF
	dc.w	$4444
