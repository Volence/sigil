; ref +; rept 2 { ++ }; file +
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	dc.w	+	; REF
	rept	2
++	dc.w	$2222
	endm
+	dc.w	$3333
	dc.w	$4444
