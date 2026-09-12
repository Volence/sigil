; body: defs + ++ + + with no bytes between them
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
+
++
+
+	dc.w	$2222
	endm
	mac
	dc.w	$4444
