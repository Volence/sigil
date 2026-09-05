	cpu	68000
	padding	off
	org	0
; --- m12/m13 showed a colon LABEL inside a rept/macro expansion is NOT #1000,
;     while the same label at file level (m3 `Fl:`) and from a twice-included
;     header (m14 `Hl:`) IS. This asks WHY: is such a label global at all? ---
	rept	1
Xr:
	dc.w	$1111
	endm
	dc.w	Xr		; if the rept label is global this resolves
mlbl	macro
Ym:
	dc.w	$2222
	endm
	mlbl
	dc.w	Ym		; if the macro label is global this resolves
; --- and the reverse direction: file-level first, expansion second ---
Zr:
	dc.w	$3333
	rept	1
Zr:
	dc.w	$3333
	endm
	dc.w	$4444
