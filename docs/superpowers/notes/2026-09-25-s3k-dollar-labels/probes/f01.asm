	cpu 68000
	org $1200
V	set	1
$$y:	nop
W	set	1
	nop
V	set	2
	dc.w $$y
