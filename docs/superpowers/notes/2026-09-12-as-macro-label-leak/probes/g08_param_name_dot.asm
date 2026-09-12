; dotted name arrives as argument text under `Base:`, read `.foo` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro	nm
nm:	dc.w	$2222
	endm
	mac	.foo
	dc.w	.foo	; REF
	dc.w	$4444
