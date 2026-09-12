; body: def + ++; file after the call: ref +, def + +
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
+	dc.w	$2222
++	dc.w	$3333
	endm
	mac
	dc.w	+	; REF
+	dc.w	$5555
+	dc.w	$6666
	dc.w	$4444
