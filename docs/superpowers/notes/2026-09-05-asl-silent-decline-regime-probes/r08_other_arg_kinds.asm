; Is the silence specific to REGISTERS, or does any argument kind asl cannot
; reduce to an integer go the same way? as.msg carries
; "expected integer, floating point number or string but got register" and
; "expected integer or string", so the catalogue distinguishes the kinds.
	cpu 68000
	padding off
fu	function p,(p*7)+$100
fi	function p,$3C7
	org $1000
	move.w	#$A101,d0
	move.w	#fu('ab'),d0	; string argument, body uses p
	move.w	#$A202,d0
	move.w	#fi('ab'),d0	; string argument, body ignores p
	move.w	#$A303,d0
	move.w	#fu(1.5),d0	; float argument, body uses p
	move.w	#$A404,d0
	move.w	#fu(zz),d0	; UNDEFINED symbol argument
	move.w	#$A505,d0
	move.w	#fi(zz),d0	; undefined symbol, body ignores p
	move.w	#$A606,d0
	move.w	#fu(),d0	; no argument at all
	move.w	#$A707,d0
	move.w	#fu(5,6),d0	; too many arguments
	move.w	#$A808,d0
	move.w	#undef_fn(a1),d0	; the function itself is undefined
	move.w	#$A909,d0
