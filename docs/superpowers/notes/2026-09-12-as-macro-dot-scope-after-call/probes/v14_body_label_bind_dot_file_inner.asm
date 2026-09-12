; file `Inner:`/`.x:`; body `Inner:`, `.v := 5`, then reads `.x`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Inner:	dc.w	$7777
.x:	dc.w	$8888
Base:	dc.w	$5555
mac	macro
Inner:	dc.w	$2222
.v	:=	5
	dc.w	.x	; REF
	endm
	mac
	dc.w	$4444
