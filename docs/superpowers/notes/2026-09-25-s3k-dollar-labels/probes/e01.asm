	cpu 68000
	org $1200
V	set	1
	nop
$$x:	nop
	dc.w $$x
V	set	2
	nop
	nop
$$x:	nop
	dc.w $$x
