; `irp` body `-` then `dc.w -`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	irp	v,1,2
-	dc.w	v
	dc.w	-	; REF
	endm
	dc.w	$4444
