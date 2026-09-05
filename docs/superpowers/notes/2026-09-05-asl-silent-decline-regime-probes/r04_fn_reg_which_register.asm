; Does WHICH register matter? Address, data, and the special names.
	cpu 68000
	padding off
fu	function p,(p*7)+$100
	org $1000
	move.w	#$A101,d0
	move.w	#fu(a0),d0
	move.w	#$A202,d0
	move.w	#fu(a7),d0
	move.w	#$A303,d0
	move.w	#fu(d3),d0
	move.w	#$A404,d0
	move.w	#fu(sp),d0
	move.w	#$A505,d0
	move.w	#fu(pc),d0
	move.w	#$A606,d0
	move.w	#fu(sr),d0
	move.w	#$A707,d0
	move.w	#fu(ccr),d0
	move.w	#$A808,d0
	move.w	#fu(usp),d0
	move.w	#$A909,d0
	move.w	#fu(a1.w),d0	; register with a size suffix
	move.w	#$AA0A,d0
	move.w	#fu(A1),d0	; case: the corpus builds with -U (case sensitive)
	move.w	#$AB0B,d0
