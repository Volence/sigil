; body: ref ++ before defs + ++ +
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	dc.w	++	; REF
+	dc.w	$2222
++	dc.w	$3333
+	dc.w	$5555
	endm
	mac
	dc.w	$4444
