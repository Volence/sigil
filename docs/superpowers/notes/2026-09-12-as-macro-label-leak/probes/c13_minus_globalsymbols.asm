; column-1 `-` in a {GLOBALSYMBOLS} body, `-` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	{GLOBALSYMBOLS}
-	dc.w	$2222
	endm
	mac
	dc.w	-	; REF
	dc.w	$4444
