; DOES AN EARLIER ERROR SWALLOW A LATER `symbol undefined`?
;
; A forward reference is legal in AS, so an undefined symbol is not an error in
; the first pass — it is a provisional value, masked to the operand's width, and
; the complaint only comes when a later pass finds it still undefined. If an
; error found EARLIER stops the pass loop, the complaint never comes: the
; provisional value is emitted, and nothing says so.
;
; This file and `r11b_no_earlier_error.asm` beside it differ in exactly one
; line — this one carries `move.w #1.5,d0`, a loud float, above the undefined
; symbols; that one carries an accepted `move.w #$B0B0,d0` in its place. Every
; other line is identical, so any difference in what asl says about `zz` is
; caused by the earlier error and by nothing else.
	cpu 68000
	padding off
	org $1000
	move.w	#$A101,d0
	move.w	#1.5,d0		; THE EARLIER ERROR
	move.w	#$A202,d0
	move.w	#zz,d0		; undefined, and never defined below
	move.w	#$A303,d0
	dc.w	zz
	move.w	#$A404,d0
