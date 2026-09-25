	cpu 68000
	org $1200
A1:	nop
	save
	cpu z80undoc
.l1:	nop
	restore
.l2:	nop
	cpu 68000
	save
	cpu z80
.l3:	nop
	restore
.l4:	nop
	supmode off
.l5:	nop
	padding on
.l6:	nop
