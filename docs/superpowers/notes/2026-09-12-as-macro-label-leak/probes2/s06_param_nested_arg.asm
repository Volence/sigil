; outer passes its argument to inner as the label name; read outside
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
inner	macro	nm
nm:	dc.w	$2222
	endm
outer	macro	q
	inner	q
	dc.w	q	; REF
	endm
	outer	Foo
	dc.w	$4444
