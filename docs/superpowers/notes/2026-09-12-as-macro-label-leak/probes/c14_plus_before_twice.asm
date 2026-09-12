; `+` before; the macro, which defines `+`, invoked twice
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
+	dc.w	$2222
	endm
	dc.w	+	; REF
	mac
	mac
	dc.w	$4444
