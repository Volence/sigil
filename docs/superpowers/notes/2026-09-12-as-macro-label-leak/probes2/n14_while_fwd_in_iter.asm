; `while` body `dc.w +` then its own `+`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
cnt	set	0
	while	cnt<2
	dc.w	+	; REF
+	dc.w	$2222
cnt	set	cnt+1
	endm
	dc.w	$4444
