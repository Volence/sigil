; outer calls inner (writes `Inner:`), then `.y:`, then `if defined(.y)` in its body picks a word
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
inner	macro
Inner:	dc.w	$2222
	endm
outer	macro
	inner
.y:	dc.w	$3333
	if	defined(.y)	; REF
	dc.w	$AAAA
	else
	dc.w	$BBBB
	endif
	endm
	outer
	dc.w	$4444
