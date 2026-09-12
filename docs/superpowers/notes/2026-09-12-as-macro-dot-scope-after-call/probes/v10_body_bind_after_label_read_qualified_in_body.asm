; body `Inner:`, `.v := 5`, then reads `Inner.v` in the body
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
Inner:	dc.w	$2222
.v	:=	5
	dc.w	Inner.v	; REF
	endm
	mac
	dc.w	$4444
