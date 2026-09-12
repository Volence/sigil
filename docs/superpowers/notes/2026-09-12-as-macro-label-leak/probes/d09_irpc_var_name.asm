; `irpc` label spelled from the loop variable by interpolation, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	irpc	c,q
L{"c"}:	dc.w	$2222
	endm
	dc.w	Lq	; REF
	dc.w	$4444
