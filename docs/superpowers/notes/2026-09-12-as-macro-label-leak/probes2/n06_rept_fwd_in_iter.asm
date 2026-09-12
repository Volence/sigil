; `rept 2` body `dc.w +` then its own `+`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	rept	2
	dc.w	+	; REF
+	dc.w	$2222
	endm
	dc.w	$4444
