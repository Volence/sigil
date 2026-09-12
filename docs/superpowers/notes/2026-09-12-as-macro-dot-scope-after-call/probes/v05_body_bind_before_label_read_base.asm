; body `.v := 5` then `Inner:`; read `Base.v` after the call
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
.v	:=	5
Inner:	dc.w	$2222
	endm
	mac
	dc.w	Base.v	; REF
	dc.w	$4444
