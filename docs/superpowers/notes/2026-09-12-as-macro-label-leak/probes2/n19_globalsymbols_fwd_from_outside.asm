; `dc.w +` before; a {GLOBALSYMBOLS} body defines `+`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	{GLOBALSYMBOLS}
+	dc.w	$2222
	endm
	dc.w	+	; REF
	mac
	dc.w	$4444
