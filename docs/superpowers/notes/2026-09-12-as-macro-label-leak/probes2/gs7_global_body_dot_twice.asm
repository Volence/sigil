; {GLOBALSYMBOLS} body `.dl:`, invoked twice under `Base:`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro	{GLOBALSYMBOLS}
.dl:	dc.w	$2222
	endm
	mac
	mac
	dc.w	$3333
	dc.w	$4444
