; `.x` read after the call, defined later in the same scope; forced passes
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	dc.w	Later
Base:	dc.w	$5555
mac	macro
Inner:	dc.w	$2222
	endm
	mac
	dc.w	.x	; REF
.x:	dc.w	$6666
Later:	dc.w	$9999
	dc.w	$4444
