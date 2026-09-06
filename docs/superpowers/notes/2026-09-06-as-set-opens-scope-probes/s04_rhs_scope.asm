; s04: is the new scope open while the binder's OWN right-hand side is
; evaluated, or does the RHS still see the previous scope?
;
; `Parent.prev` exists; `Vr.prev` never does.  So the RHS `.prev` resolves
; under exactly one candidate parent and the bound value says which:
;   $1002  ->  RHS evaluated in the OLD scope (Parent)
;   error  ->  RHS evaluated in the NEW scope (Vr), where `.prev` is undefined
	cpu	68000
	padding	off
	org	$1000
Parent:
	nop
.prev:
	nop
Vr	set	.prev
Eqr	equ	.prev
.after:
	nop
	end
