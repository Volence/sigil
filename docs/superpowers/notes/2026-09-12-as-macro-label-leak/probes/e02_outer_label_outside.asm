; outer body label (outer also calls inner), read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
inner	macro
	dc.w	$3333
	endm
outer	macro
Lo:	dc.w	$2222
	inner
	endm
	outer
	dc.w	Lo	; REF
	dc.w	$4444
