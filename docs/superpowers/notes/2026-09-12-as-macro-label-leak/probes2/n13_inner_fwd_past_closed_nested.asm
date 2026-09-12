; outer: `dc.w +`, calls inner (defines `+`), then defines its own `+`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
inner	macro
+	dc.w	$2222
	endm
outer	macro
	dc.w	+	; REF
	inner
+	dc.w	$3333
	endm
	outer
	dc.w	$4444
