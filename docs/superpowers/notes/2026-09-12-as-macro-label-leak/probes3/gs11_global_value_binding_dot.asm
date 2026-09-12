; {GLOBALSYMBOLS} body `.v := 7` under Base, read `Base.v` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro	{GLOBALSYMBOLS}
.v	:=	7
	endm
	mac
	dc.w	Base.v	; REF
	dc.w	$4444
