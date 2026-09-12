; plain macro called `Lbl mac`, body `Inner:`; after `.b := 2` read `Lbl.b`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
Inner:	dc.w	$2222
	endm
Lbl	mac
.b	:=	2
	dc.w	Lbl.b	; REF
	dc.w	$4444
