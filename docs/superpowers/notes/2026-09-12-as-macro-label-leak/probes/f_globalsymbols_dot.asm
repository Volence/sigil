; {GLOBALSYMBOLS} body `.dl:` under `Base:`, read `.dl` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro	{GLOBALSYMBOLS}
.dl:	dc.w	$2222
	endm
	mac
	dc.w	.dl	; REF
	dc.w	$4444
