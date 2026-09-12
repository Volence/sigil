; four - defs, dbf d0,----
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
-	dc.w	$2222
-	dc.w	$3333
-	dc.w	$5555
-	dc.w	$6666
	dbf	d0,----	; REF
	dc.w	$4444
