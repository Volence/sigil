	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	phase $3400
	nop
	dc.w $$x
$$y:	dc.w $$y
	dephase
	dc.w $$x,$$y
