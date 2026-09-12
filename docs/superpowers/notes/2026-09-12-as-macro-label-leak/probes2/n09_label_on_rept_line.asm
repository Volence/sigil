; the corpus shape: `-` on the `rept` line, `dbf d0,-` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	moveq	#1,d0
-	rept	2
	nop
	endm
	dbf	d0,-	; REF
	dc.w	$4444
