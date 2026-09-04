	cpu 68000
	padding off
	org $0
HorizontalScrollBuffer struct dots
	ds.l	224	; Total lines on the screen.
	ds.l	16	; A bug/optimisation.
	ds.b	$40	; These are just unused.
HorizontalScrollBuffer endstruct
	dc.l HorizontalScrollBuffer.len
