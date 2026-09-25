	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	section S1
	nop
	dc.w $$x
	endsection S1
	dc.w $$x
