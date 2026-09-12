; outer passes its argument to inner as the label name; read at file level
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
inner	macro	nm
nm:	dc.w	$2222
	endm
outer	macro	q
	inner	q
	endm
	outer	Foo
	dc.w	Foo	; REF
	dc.w	$4444
