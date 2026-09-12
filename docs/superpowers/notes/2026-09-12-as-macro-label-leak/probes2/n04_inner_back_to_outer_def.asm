; outer defines `-` then calls inner, which references `-`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
inner	macro
	dc.w	-	; REF
	endm
outer	macro
-	dc.w	$2222
	inner
	endm
	outer
	outer
	dc.w	$4444
