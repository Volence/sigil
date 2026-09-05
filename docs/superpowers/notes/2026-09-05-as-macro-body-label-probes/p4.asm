	cpu	68000
	padding	off
	org	0
; Q4b: NESTED, the other direction — an OUTER macro's label read from inside
; the INNER macro it calls. Invoked twice; addresses differ ($0 vs $4).
inner4	macro
	dc.w	No
	endm
outer4	macro
No:	dc.w	$2222
	inner4
	endm
	outer4
	outer4
	dc.w	$4444
