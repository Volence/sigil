	cpu	68000
	padding	off
	org	0
; --- label (colon) then X ---
Al:
Al	equ	2
	dc.w	$1111
Bl:
Bl	=	2
	dc.w	$1111
Cl:
Cl	set	2
	dc.w	$1111
Dl:
Dl	eval	2
	dc.w	$1111
El:
El	:=	2
	dc.w	$1111
Fl:
Fl:
	dc.w	$1111
	dc.w	$4444
