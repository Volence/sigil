; The control for `remote_error_placeholder.asm`: the same file without the
; `zzbogus` line, and nothing else changed.
; asl: 2 passes, 0 errors, exit 0, and the macro's `beq.s +` reads 6702.
	cpu	68000
	org	$1000
m	macro
	tst.w	d0
	beq.s	+
	nop
+
	endm
	m
	rts
	end
