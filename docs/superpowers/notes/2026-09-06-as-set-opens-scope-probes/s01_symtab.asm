; s01: WHICH parent does a `.local` after a `set` attach to?
; Read from asl's own SYMBOL TABLE, so no reference is needed and the run
; exits 0 with a COMPLETE pass loop.  A probe that references the local
; cannot have both: one of the two candidate spellings is undefined, that
; is an error, and an error stops asl's pass loop.
;
; MUST FAIL if `set` opens no scope: the table would then read `Parent.lq`.
	cpu	68000
	padding	off
	org	$1000
Parent:
	nop
Var:	set	5
.lq:
	nop
	end
