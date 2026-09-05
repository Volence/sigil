; CONTROL for r01. An UNDEFINED symbol in the same places a register was silent.
;
; If `dc.w zz` (zz undefined) is loud while `dc.w a1` is silent, the silence in
; r01 is a property of the REGISTER kind, not of the storage directive.
	cpu 68000
	padding off
	org $1000
	move.w	#$A101,d0
	move.w	#zz,d0		; immediate, undefined symbol
	move.w	#$A202,d0
	dc.w	zz		; storage directive, undefined symbol
	move.w	#$A303,d0
	dc.b	zz
	move.w	#$A404,d0
	dc.l	zz
	move.w	#$A505,d0
	dc.w	zz+0
	move.w	#$A606,d0
