; s06: a binder whose DECLARATION asl refuses (a class crossing) -- does the
; scope still open?
;
; This run FAILS on purpose (`#2030 constants cannot be redefined as
; variables`), so its byte column is not a source of values and its diagnostic
; set is INCOMPLETE.  Only the SYMBOL TABLE is read, and only for the spelling
; of `.rr`, which is a definition rather than a reference and so does not
; depend on a later pass.
	cpu	68000
	padding	off
	org	$1000
Anchor:
	nop
Kc	equ	9
Kc	set	10
.rr:
	nop
	end
