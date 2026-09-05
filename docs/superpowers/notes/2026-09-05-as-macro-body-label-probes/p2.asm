	cpu	68000
	padding	off
	org	0
; Q3, FORWARD direction: the `end-start` shape s2's macrosetup actually uses,
; and the one the census found 39 sites of. The two expansions have DIFFERENT
; sizes (n=1 then n=3), so Ef-Sf is $0003 in the first and $0005 in the second.
; A global binding whose second expansion overwrote the first cannot produce
; that pair.
mf	macro	n
Sf:	ds.b	n
	dc.w	Ef-Sf
Ef:
	endm
	mf	1
	mf	3
	dc.w	$4444
