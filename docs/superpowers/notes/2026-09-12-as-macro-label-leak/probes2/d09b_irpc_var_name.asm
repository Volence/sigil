; `irpc` label spelled by the loop variable, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	irpc	c,"Qz"
c:	dc.w	$2222
	endm
	dc.w	Q	; REF
	dc.w	$4444
