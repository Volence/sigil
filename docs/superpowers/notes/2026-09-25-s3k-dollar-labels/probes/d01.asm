	cpu 68000
	org $1200
A1:	dc.w $$x
	nop
$$x:	dc.w $$x,$$y
$$y:	dc.w $$y
B1:	nop
	nop
	nop
$$x:	dc.w $$x
	dc.w $$y
$$y:	dc.w $$x,$$y
