; outer defines `-`; inner (called after) does `rept 1` whose body refs `-`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
inner	macro
	rept	1
	dc.w	-	; REF
	endm
	endm
outer	macro
-	dc.w	$2222
	inner
	endm
	outer
	dc.w	$4444
