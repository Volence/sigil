	cpu 68000
	org $1000
	include "s1.sounddriver.ram.asm"
	dc.l SMPS_Track.len, SMPS_RAM.len
