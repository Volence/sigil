	cpu 68000
	org $1200
A1:	nop
	section S1
	nop
$$y:	nop
	endsection S1
	dc.w $$y
