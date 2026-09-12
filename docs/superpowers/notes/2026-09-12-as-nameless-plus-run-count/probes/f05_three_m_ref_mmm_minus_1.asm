; three - defs, ref ---1 read as (--) - 1... no: (---)-1 needs ----1
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
-	dc.w	$2222
-	dc.w	$3333
-	dc.w	$5555
	dc.w	----1	; REF
	dc.w	$4444
