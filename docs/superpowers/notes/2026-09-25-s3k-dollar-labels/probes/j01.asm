	cpu 68000
	org $1200
A1:	nop
	dc.w defined($$x)
$$x:	nop
$$v	set	5
	dc.w defined($$x),defined($$v),defined(A1)
	if defined($$v)
	dc.w $1111
	else
	dc.w $2222
	endif
