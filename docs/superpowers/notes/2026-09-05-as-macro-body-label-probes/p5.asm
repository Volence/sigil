	cpu	68000
	padding	off
	org	0
; Q4c: a label defined in ONE macro's expansion read from a DIFFERENT macro's
; expansion. Both reads are "inside an expansion", but not inside the SAME
; expansion — the cell that separates "local to the expansion instance" from
; "visible anywhere inside any expansion". Each macro runs twice and the two
; definitions sit at different addresses ($0 and $6).
d5a	macro
Dx:	dc.w	$1111
	endm
d5b	macro
	dc.w	Dx
	endm
	d5a
	d5b
	d5a
	d5b
	dc.w	$4444
