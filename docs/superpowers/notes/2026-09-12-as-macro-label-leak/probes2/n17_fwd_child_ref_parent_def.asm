; outer: calls inner (whose rept body refs `+`), then outer defines `+`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
inner	macro
	rept	1
	dc.w	+	; REF
	endm
	endm
outer	macro
	inner
+	dc.w	$2222
	endm
	outer
	dc.w	$4444
