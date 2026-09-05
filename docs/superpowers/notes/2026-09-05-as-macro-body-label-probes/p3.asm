	cpu	68000
	padding	off
	org	0
; Q4a: NESTED — an INNER macro's label read by the OUTER macro that called it.
; Invoked twice, and the label's address differs between the two ($0 and $4),
; so "resolves to the wrong expansion" is a distinguishable third answer, not
; folded into "resolves".
inner3	macro
Ni:	dc.w	$1111
	endm
outer3	macro
	inner3
	dc.w	Ni
	endm
	outer3
	outer3
	dc.w	$4444
