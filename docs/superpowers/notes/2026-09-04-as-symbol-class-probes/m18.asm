	cpu	68000
	padding	off
	org	0
; m12/m13/m17 established that a colon LABEL inside a macro/rept expansion is
; expansion-local (accepted twice) while an `equ` in the same position is not
; (`#1000` on the second). This asks the same of the REMAINING constant-making
; forms, so the exemption is drawn around what was measured rather than around
; a guess about "labels".
; --- the `label` directive inside a macro invoked twice ---
mlabdir	macro
Al	label	$100
	endm
	mlabdir
	mlabdir
; --- an `enum` member inside a macro invoked twice ---
menum	macro
	enum	Be=5
	endm
	menum
	menum
; --- a bare, column-0 (colon-less) label inside a macro invoked twice ---
mbare	macro
Cl
	dc.w	$1111
	endm
	mbare
	mbare
; --- a label on a DATA line inside a macro invoked twice ---
mdata	macro
Dl:	dc.w	$2222
	endm
	mdata
	mdata
; --- and the other two expansion drivers, with a colon label ---
	irp	n,1,2
El:
	dc.w	n
	endm
Wc	set	0
	while	Wc<2
Fl:
	dc.w	$3333
Wc	set	Wc+1
	endm
	dc.w	$4444
