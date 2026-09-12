; `bra.s .x` before the call; `.x:` after it; then a second `Base2:`/`.x:` control
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
Inner:	dc.w	$2222
	endm
	bra.s	.x	; REF
	mac
.x:	dc.w	$6666
Base2:	dc.w	$9999
.x:	dc.w	$9998
	dc.w	$4444
