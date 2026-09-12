; file -; rept 1 { / }; file -; ref --
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
-	dc.w	$2222
	rept	1
/	dc.w	$3333
	endm
-	dc.w	$5555
	dc.w	--	; REF
	dc.w	$4444
