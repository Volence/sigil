; `irp` label spelled by the loop variable, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	irp	v,Aa
v:	dc.w	$2222
	endm
	dc.w	Aa	; REF
	dc.w	$4444
