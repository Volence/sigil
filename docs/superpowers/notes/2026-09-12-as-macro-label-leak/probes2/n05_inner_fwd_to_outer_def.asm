; inner references `+`; outer calls inner then defines `+`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
inner	macro
	dc.w	+	; REF
	endm
outer	macro
	inner
+	dc.w	$2222
	endm
	outer
	outer
	dc.w	$4444
