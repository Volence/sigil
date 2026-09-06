; WHY 67FE IS NOT "THE ERROR CHANGING ANOTHER LINE".
;
; `../asl-reference/partial_failure.asm` shows a macro's `beq.s +` reading 67FE
; (a branch to itself) with a bad `bra.s /` line present and 6702 (the correct
; forward branch) with it deleted. The committed prose used to attribute that to
; the error corrupting the neighbouring line.
;
; This file separates the two readings. There is no `bra.s /` anywhere. The
; error is an unknown instruction on line 3, ABOVE the macro definition, sharing
; nothing with the branch. `remote_error_control.asm` beside it is this file
; minus that one line.
;
;     this file                 beq.s + reads 67FE, 1 pass, warning present
;     remote_error_control.asm  beq.s + reads 6702, 2 passes, exit 0
;
; So 67FE is the PASS-1 PLACEHOLDER for a forward reference that pass 2 would
; have resolved, and it survives because the error stopped the pass loop. Any
; error, at any position, related or not, does this to every forward reference
; in the file.
	cpu	68000
	org	$1000
	zzbogus	d0,d1
m	macro
	tst.w	d0
	beq.s	+
	nop
+
	endm
	m
	rts
	end
