	cpu 68000
	movem.w T(pc),d2-d3
	movem.l (T,pc),d0/a1
T: dc.w 1
	movem.w T(pc),a2/d6
