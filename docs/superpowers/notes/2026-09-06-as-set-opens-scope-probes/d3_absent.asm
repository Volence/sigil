; d3: a control for the two direction probes.  `Parent.nothere` is defined
; NOWHERE, so an assembler that refuses d1 for the right reason must refuse
; this too, and one that assembles d1 through some fallback path assembles
; this as well.  Without it, "d1 assembles" has two readings.
	cpu	68000
	padding	off
	org	$1000
Parent:
	nop
Var	set	5
.lq:
	nop
	dc.l	Parent.nothere
	end
