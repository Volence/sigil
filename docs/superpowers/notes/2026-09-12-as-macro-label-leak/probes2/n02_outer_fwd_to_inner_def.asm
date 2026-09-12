; outer body `dc.w +` then calls inner, which defines `+`
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
	endm
	outer
+	dc.w	$3333
	dc.w	$4444
