; inner macro's label, read at file level after the outer returns
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
inner	macro
Ln:	dc.w	$2222
	endm
outer	macro
	inner
	endm
	outer
	dc.w	Ln	; REF
	dc.w	$4444
