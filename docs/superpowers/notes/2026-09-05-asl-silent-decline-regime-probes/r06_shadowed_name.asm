; A register name that is ALSO a defined symbol. Can it even be defined, and if
; so does the definition win inside a function-call argument?
;
; $77 and $2A share no digits with any function value here, so no two readings
; produce the same word.
	cpu 68000
	padding off
fu	function p,(p*7)+$100
a1	=	$77
d3	=	$2A
	org $1000
	move.w	#$A101,d0
	move.w	#fu(a1),d0	; if the equate wins: $77*7+$100 = $4A9
	move.w	#$A202,d0
	move.w	#fu(d3),d0	; if the equate wins: $2A*7+$100 = $226
	move.w	#$A303,d0
	move.w	#a1,d0		; the bare name, immediate
	move.w	#$A404,d0
	dc.w	a1
	move.w	#$A505,d0
	move.w	#$1234,a1(a2)	; the disp-or-call shape with the shadowed name
	move.w	#$A606,d0
