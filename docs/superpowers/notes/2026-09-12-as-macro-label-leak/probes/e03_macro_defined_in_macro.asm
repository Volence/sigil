; a macro DEFINED inside a macro body, invoked at file level, its label read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
outer	macro
inn	macro
Ld:	dc.w	$2222
	endm
	endm
	outer
	inn
	dc.w	Ld	; REF
	dc.w	$4444
