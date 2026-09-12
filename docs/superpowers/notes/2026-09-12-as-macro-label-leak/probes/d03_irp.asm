; plain label in a file-level `irp` body, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	irp	v,5
Li:	dc.w	$2222
	endm
	dc.w	Li	; REF
	dc.w	$4444
