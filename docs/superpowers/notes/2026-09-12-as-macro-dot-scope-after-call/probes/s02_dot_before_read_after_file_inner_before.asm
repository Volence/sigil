; file-level `Inner:`/`.x:` first; `.x:` under Base; call; read `.x` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Inner:	dc.w	$7777
.x:	dc.w	$8888
Base:	dc.w	$5555
.x:	dc.w	$6666
mac	macro
Inner:	dc.w	$2222
	endm
	mac
	dc.w	.x	; REF
	dc.w	$4444
