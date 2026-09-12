; bra.s ++ before defs + ++ +
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	bra.s	++	; REF
+	dc.w	$2222
++	dc.w	$3333
+	dc.w	$5555
	dc.w	$4444
