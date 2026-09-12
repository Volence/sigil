; body `Inner:`; after the call `.y:`; read `Base.y`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
Inner:	dc.w	$2222
	endm
	mac
.y:	dc.w	$6666
	dc.w	Base.y	; REF
	dc.w	$4444
