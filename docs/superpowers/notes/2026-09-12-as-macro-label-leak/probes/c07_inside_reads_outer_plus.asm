; the body references `+`; outer `+` after the call
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	dc.w	+	; REF
	endm
	mac
+	dc.w	$3333
	dc.w	$4444
