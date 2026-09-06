; partial_failure.asm - THE DANGEROUS SHAPE, as a fixture.
;
; This file MOSTLY ASSEMBLES. asl reports one error, exits non-zero, and still
; prints a full byte column for every line - including a line whose value the
; error CHANGED. That is the case a reader trusts and should not: a file that
; fails to assemble at all announces itself, and this one does not.
;
; The error is `bra.s /`. In AS, `/` is a nameless label DEFINITION only; it is
; not a reference, so the branch has no target. Everything else here is valid.
;
; The corrupted line is the macro's `beq.s +`. Assembled with the `bra.s /` line
; present it comes back as a branch to ITSELF; assembled without it, it comes
; back as the correct forward branch over the `nop`. Nothing in the listing says
; which of the two you are reading. `selfcheck.sh` asserts both halves.
;
; The branch-to-itself is the PASS-1 PLACEHOLDER for a forward reference, not a
; value the bad line computed. Any error anywhere stops asl's pass loop, and the
; pass that would have resolved the branch never runs; an unrelated error on the
; line above the macro does the same thing. That is also why this file's footer
; carries `Additional necessary passes not started`, and why any undefined symbol
; added below would be reported as ZERO.
	cpu	68000
	org	$1000
m	macro
	tst.w	d0
	beq.s	+
	nop
+
	endm
	m
	bra.s	/
	rts
	end
