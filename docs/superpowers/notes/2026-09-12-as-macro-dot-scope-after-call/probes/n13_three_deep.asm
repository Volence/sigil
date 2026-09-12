; a calls b calls c, c writes `Deep:`; after a `.b := 2` read `Deep.b`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mc	macro
Deep:	dc.w	$2222
	endm
mb	macro
	mc
	endm
ma	macro
	mb
	endm
	ma
.b	:=	2
	dc.w	Deep.b	; REF
	dc.w	$4444
