; outer calls inner (defines `-`), then outer references `-`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
inner	macro
-	dc.w	$2222
	endm
outer	macro
	inner
	dc.w	-	; REF
	endm
	outer
	dc.w	$4444
