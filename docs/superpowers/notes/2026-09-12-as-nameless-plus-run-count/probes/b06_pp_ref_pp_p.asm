; def ++, ref ++, def +
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
++	dc.w	$2222
	dc.w	++	; REF
+	dc.w	$3333
	dc.w	$4444
