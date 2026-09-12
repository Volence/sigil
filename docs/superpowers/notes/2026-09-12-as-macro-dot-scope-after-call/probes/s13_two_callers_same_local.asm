; two routines, each calls mac then defines and reads `.lp` (collide as Inner.lp?)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
Inner:	dc.w	$2222
	endm
R1:	dc.w	$5555
	mac
.lp:	dc.w	$6666
	dc.w	.lp
R2:	dc.w	$5556
	mac
.lp:	dc.w	$6667
	dc.w	.lp	; REF
	dc.w	$4444
