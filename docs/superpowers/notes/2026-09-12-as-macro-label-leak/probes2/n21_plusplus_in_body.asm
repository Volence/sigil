; body `++`; `+++` referenced before the call, `+` after the call defined outside
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
++	dc.w	$2222
	endm
	dc.w	+++	; REF
	mac
+	dc.w	$3333
	dc.w	$4444
