; body defines `-` then emits its argument; called with `-` (whose `-`?)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
-	dc.w	$3333
mac	macro	v
-	dc.w	$2222
	dc.w	v	; REF
	endm
	mac	-
	dc.w	$4444
