	cpu 68000
	padding off
	org 0
	move.w	1(a0),d0
	move.w	1(a0,d0.w),d0
	move.w	(1,a0),d0
	move.w	d0,-1(a1)
	move.l	3(sp),d0
