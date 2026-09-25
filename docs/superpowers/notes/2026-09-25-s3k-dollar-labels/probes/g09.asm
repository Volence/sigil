	cpu 68000
	org $1200
V	set	$12
$$x:	nop
	pushv	V
	dc.w $$x
	popv	V
	dc.w $$x
