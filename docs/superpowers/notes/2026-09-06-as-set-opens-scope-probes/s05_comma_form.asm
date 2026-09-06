; s05: the OPERAND-field spellings `set NAME,value` and `eval NAME,value`.
; The name is not in asl's label field there.  Does it still open a scope?
;
; `Cf.mm` versus `Anchor.mm`, and `Ef.nn` versus `Anchor.nn` (or `Cf.nn`
; if the first form did open one).  Each local name occurs once.
	cpu	68000
	padding	off
	org	$1000
Anchor:
	nop
	set	Cf,5
.mm:
	nop
	eval	Ef,6
.nn:
	nop
	end
