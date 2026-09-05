; Where is a REGISTER USED AS A VALUE loud, and where is it silent?
;
; Every suspect line is preceded by an accepted `move.w #$Annn,d0` whose word is
; unique, so a suspect that echoes the setter above it has been SUBSTITUTED (asl
; declined to value it) rather than answered. That is the d9 carry technique with
; a plain constant as the setter instead of a function call.
	cpu 68000
	padding off
	org $1000
	move.w	#$A101,d0
	move.w	#a1,d0		; immediate, bare register
	move.w	#$A202,d0
	move.w	#1+a1,d0	; immediate, register inside an expression
	move.w	#$A303,d0
	dc.w	a1		; storage directive, bare register
	move.w	#$A404,d0
	dc.b	a1
	move.w	#$A505,d0
	dc.l	a1
	move.w	#$A606,d0
sym	=	a1		; equate whose value is a register
	move.w	#$A707,d0
	dc.w	sym
	move.w	#$A808,d0
	dc.w	a1+0		; register inside an expression, storage directive
	move.w	#$A909,d0
