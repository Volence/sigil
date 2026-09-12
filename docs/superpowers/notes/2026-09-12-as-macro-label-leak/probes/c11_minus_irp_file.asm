; column-1 `-` inside a file-level `irp` body, `-` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	irp	v,5
-	dc.w	$2222
	endm
	dc.w	-	; REF
	dc.w	$4444
