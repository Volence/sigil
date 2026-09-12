; file ++; body +; file + (lands on the file ++'s slot)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
++	dc.w	$2222
mac	macro
+	dc.w	$3333
	endm
	mac
+	dc.w	$5555
	dc.w	$4444
