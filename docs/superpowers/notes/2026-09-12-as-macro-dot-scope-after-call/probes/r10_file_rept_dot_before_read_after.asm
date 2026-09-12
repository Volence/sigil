; `.x:` under Base; file-level `rept 1` / `Inner:`; read `.x` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
.x:	dc.w	$6666
	rept	1
Inner:	dc.w	$2222
	endr
	dc.w	.x	; REF
	dc.w	$4444
