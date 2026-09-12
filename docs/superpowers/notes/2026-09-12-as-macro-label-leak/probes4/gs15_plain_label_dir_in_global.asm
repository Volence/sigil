; {GLOBALSYMBOLS} outer calls plain inner writing `Tbl label *`; after, `.c := 3`, read `Tbl.c`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
inner	macro
Tbl	label	*
	endm
outer	macro	{GLOBALSYMBOLS}
	inner
	endm
	outer
.c	:=	3
	dc.w	Tbl.c	; REF
	dc.w	$4444
