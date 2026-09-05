; `f(<register>)` in EVERY context that takes an expression, not only in an
; immediate. Which of them are silent?
;
; `fu` uses its parameter; `fi` ignores it. Both are exercised, because a body
; that ignores its parameter can fold without ever looking at the argument.
	cpu 68000
	padding off
fu	function p,(p*7)+$100
fi	function p,$3C7
	org $1000
	move.w	#$A101,d0
	move.w	#fu(a1),d0	; immediate
	move.w	#$A202,d0
	move.w	#fi(a1),d0	; immediate, body ignores p
	move.w	#$A303,d0
	dc.w	fu(a1)		; storage directive
	move.w	#$A404,d0
	dc.w	fi(a1)
	move.w	#$A505,d0
eq	=	fu(a1)		; equate
	move.w	#$A606,d0
	dc.w	eq
	move.w	#$A707,d0
	move.w	#$1234,fu(a1)	; DESTINATION operand — the disp-or-call shape
	move.w	#$A808,d0
	move.w	#$1234,1+fu(a1)	; the insn2op arm
	move.w	#$A909,d0
	move.w	fu(a1),d0	; SOURCE operand
	move.w	#$AA0A,d0
	move.w	#fu(5),d0	; control: a real argument, must be $123
	move.w	#$AB0B,d0
