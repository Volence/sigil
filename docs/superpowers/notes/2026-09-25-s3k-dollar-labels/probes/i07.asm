	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	dc.w defined($$x),defined($$y)
	ifdef $$x
	dc.w $1111
	endif
	ifndef $$y
	dc.w $2222
	endif
