; body /; file + -; file ref - (x7 with a ref)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
/	dc.w	$2222
	endm
	mac
+	dc.w	$3333
-	dc.w	$5555
	dc.w	-	; REF
	dc.w	$4444
