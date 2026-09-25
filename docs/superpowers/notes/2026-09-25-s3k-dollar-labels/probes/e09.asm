	cpu 68000
	org $1200
A1:	nop
$$v	set	$1234
	dc.w $$v
$$v	set	$2345
	dc.w $$v
