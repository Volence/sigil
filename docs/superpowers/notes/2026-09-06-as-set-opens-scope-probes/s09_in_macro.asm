; s09: a GLOBAL-named `set` inside a macro expansion.  Which scope does the
; local written after it in the CALLER attach to?
;
; A macro's own body gets an unspellable expansion scope in asl, so this asks
; whether the scope a `set` opens survives the expansion the way a plain label
; in a macro body does.
	cpu	68000
	padding	off
	org	$1000
opener	macro
Ms	set	5
	endm
Anchor:
	nop
	opener
.uu:
	nop
	end
