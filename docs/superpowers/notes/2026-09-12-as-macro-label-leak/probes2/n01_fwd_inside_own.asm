; body `dc.w +` then its own `+`, invoked twice
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	dc.w	+	; REF
+	dc.w	$2222
	endm
	mac
	mac
	dc.w	$4444
