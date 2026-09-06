; swallowed_undef_control.asm - the control for `swallowed_undef.asm`.
;
; Identical to it except that the `zzbogus d0,d1` line is absent. The same three
; undefined symbols are here, and asl reports all three: `3 errors`, two passes,
; and NO `Additional necessary passes not started` line in the footer.
;
; This file is also the case that keeps the check honest. IT FAILS TOO - exit 2,
; three errors - and its diagnostics are COMPLETE. A detector keyed to "the run
; failed" would fire on it; the detector in `asl_ref.sh` must not, or it becomes
; the always-red kind that people learn to switch off.
	cpu	68000
	org	$1000
start:
	move.w	#UND_ONE,d0
	move.w	#UND_TWO,d1
	move.w	#UND_THREE,d2
	rts
	end
