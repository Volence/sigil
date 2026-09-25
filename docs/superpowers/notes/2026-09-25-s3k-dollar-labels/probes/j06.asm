	cpu 68000
	org $1200
V	set	$10
$$x:	nop
W	set	$20
	nop
V	set	$30
	dc.w $$x
	nop
$$y:	nop
W	set	$40
	nop
V	set	$50
	dc.w $$y
