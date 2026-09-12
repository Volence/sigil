; `.x:` under Base before the call; read `Base.x` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
.x:	dc.w	$6666
mac	macro
Inner:	dc.w	$2222
	endm
	mac
	dc.w	Base.x	; REF
	dc.w	$4444
