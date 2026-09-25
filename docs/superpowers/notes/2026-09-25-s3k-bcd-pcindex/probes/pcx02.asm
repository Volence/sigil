	cpu 68000
B: dc.w $1111,$2222
	move.w B(pc,d3.w),d5
	movem.w B(pc,d0.w),d2-d3
	lea B(pc,a3.l),a5
	jmp B(pc,d6.w)
	btst #7,B(pc,d3.w)
