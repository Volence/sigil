; `.x:` under Base; outer calls inner, then reads `.x` in its body
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
.x:	dc.w	$6666
inner	macro
Inner:	dc.w	$2222
	endm
outer	macro
	inner
	dc.w	.x	; REF
	endm
	outer
	dc.w	$4444
