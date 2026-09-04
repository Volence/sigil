	cpu 68000
	org 0
v_ram:	equ $FFFF0000
v_ctl:	equ $FFFFF700
	move.w	#v_ram,d0
	move.w	#v_ctl,d0
	move.w	#-65536,d1
	move.w	#$FFFF0000,d1
	andi.w	#v_ctl,d2
	dc.w	v_ram
	dc.w	v_ctl
	end
