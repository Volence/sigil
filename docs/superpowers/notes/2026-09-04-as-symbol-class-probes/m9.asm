	cpu	68000
	padding	off
	org	0
; --- variable FIRST, then each of the four constant-making forms ---
Av	set	1
	dc.w	$1111
Av:
	dc.w	$1111
Bv	set	1
	dc.w	$1111
Bv
	dc.w	$1111
Cv	set	1
Cv	label	$100
Dv	set	1
	enum	Dv=5
; --- does the refused label still open a local scope? ---
.aft	equ	7
	dc.w	.aft
	dc.w	$4444
