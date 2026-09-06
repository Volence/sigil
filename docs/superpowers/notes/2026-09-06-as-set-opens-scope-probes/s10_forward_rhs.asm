; s10: a binder whose RHS is a FORWARD reference -- undefined on pass 1, defined
; on pass 2.  Does the scope open?
;
; This is the shape that decides whether "the scope opens when the value binds"
; is pass-stable: if it opened only on the pass where the RHS resolved, the
; qualification of `.vv` would differ between passes.  The run exits 0, so the
; table is the settled answer.
	cpu	68000
	padding	off
	org	$1000
Anchor:
	nop
Fw	set	Later
.vv:
	nop
Later:
	nop
	end
