; s08: a binder whose RIGHT-HAND SIDE does not evaluate.  Does it open a scope?
;
; This run FAILS on purpose, so only the symbol table's spelling of `.tt` is
; read, and that is a DEFINITION, decided on the pass that reaches it.
;   Bd.tt      ->  the scope opens from the label field regardless of the value
;   Anchor.tt  ->  no value, no scope
	cpu	68000
	padding	off
	org	$1000
Anchor:
	nop
Bd	set	nosuchsymbol
.tt:
	nop
	end
