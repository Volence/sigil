	cpu	68000
	padding	off
	org	0
; The direction that must NOT break: a FILE-LEVEL label read from inside an
; expansion, both backward and forward. This is the ubiquitous case — if it
; stopped resolving, nothing would assemble — so it is here as the control that
; says the exemption is drawn around the definition site, not the read site.
Gb:
	dc.w	$1111
mread	macro
	dc.w	Gb
	dc.w	Gf
	endm
	mread
	mread
Gf:
	dc.w	$4444
