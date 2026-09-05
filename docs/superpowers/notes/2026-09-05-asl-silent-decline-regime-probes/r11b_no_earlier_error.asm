; The control for `r11_earlier_error_swallows_undef.asm`. One line differs: the
; loud `move.w #1.5,d0` is replaced by an accepted `move.w #$B0B0,d0`. Every
; other line, including both undefined-symbol lines, is identical.
	cpu 68000
	padding off
	org $1000
	move.w	#$A101,d0
	move.w	#$B0B0,d0	; the earlier error, REMOVED
	move.w	#$A202,d0
	move.w	#zz,d0		; undefined, and never defined below
	move.w	#$A303,d0
	dc.w	zz
	move.w	#$A404,d0
