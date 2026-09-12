; file-level `-`, then a macro whose body emits its argument, called with `-`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
-	dc.w	$3333
mac	macro	v
	dc.w	v	; REF
	endm
	mac	-
	dc.w	$4444
