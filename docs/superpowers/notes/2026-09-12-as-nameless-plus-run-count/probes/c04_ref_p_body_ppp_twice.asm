; ref + before two calls of body +++; file + after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	dc.w	+	; REF
mac	macro
+++	dc.w	$2222
	endm
	mac
	mac
+	dc.w	$3333
	dc.w	$4444
