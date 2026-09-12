; file-level `rept 1` / `Inner:`; after `.b := 2` read `Base.b`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
	rept	1
Inner:	dc.w	$2222
	endr
.b	:=	2
	dc.w	Base.b	; REF
	dc.w	$4444
