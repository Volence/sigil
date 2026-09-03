	cpu	68000
	padding	off
	phase	$FFFF0000
A:	ds.b	$B026
Engine_RAM_End:
	dephase
	phase	Engine_RAM_End
	ds.b	4
	align	256
Player_Pos_Ring:	ds.b	256
	dc.w	Player_Pos_Ring
	dephase
