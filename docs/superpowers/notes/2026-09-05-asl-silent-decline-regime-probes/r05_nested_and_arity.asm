; Nesting, arity, and the register buried one level down.
	cpu 68000
	padding off
fu	function p,(p*7)+$100
g	function q,q+$10
f2	function p,q,(p*7)+q
	org $1000
	move.w	#$A101,d0
	move.w	#fu(g(a1)),d0	; register is the INNER call's argument
	move.w	#$A202,d0
	move.w	#g(fu(a1)),d0	; register is the inner call's argument, other order
	move.w	#$A303,d0
	move.w	#fu(1+a1),d0	; register inside the argument EXPRESSION
	move.w	#$A404,d0
	move.w	#fu(fu(5)),d0	; control: nesting with no register at all
	move.w	#$A505,d0
	move.w	#f2(a1,5),d0	; two-argument, register first
	move.w	#$A606,d0
	move.w	#f2(5,a1),d0	; two-argument, register second
	move.w	#$A707,d0
	move.w	#f2(5,7),d0	; control: two-argument, no register
	move.w	#$A808,d0
	move.w	#fu(a1)+fu(5),d0	; declined call ADDED to an accepted one
	move.w	#$A909,d0
