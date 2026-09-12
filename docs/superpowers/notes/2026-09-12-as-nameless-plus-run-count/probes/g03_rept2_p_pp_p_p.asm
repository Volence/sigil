; rept 2 { defs + ++ + + }
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	rept	2
+	dc.w	$2222
++	dc.w	$3333
+	dc.w	$5555
+	dc.w	$6666
	endm
	dc.w	$4444
