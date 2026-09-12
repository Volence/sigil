; body `Inner:` reads `.x`; the file-level `Inner:`/`.x:` comes after the call
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
Inner:	dc.w	$2222
	dc.w	.x	; REF
	endm
	mac
Inner:	dc.w	$7777
.x:	dc.w	$8888
	dc.w	$4444
