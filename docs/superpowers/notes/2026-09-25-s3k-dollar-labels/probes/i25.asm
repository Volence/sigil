	cpu 68000
	org $1200
inner	macro
	nop
	dc.w $$a
	endm
outer	macro
	nop
$$a:	nop
	inner
	endm
A1:	nop
$$a:	nop
	outer
	outer
	dc.w $$a
