	cpu	68000
	padding	off
	org	0
; The `label` DIRECTIVE read from INSIDE the body that declares it. m18/m19
; measured it as GLOBAL and readable from OUTSIDE; this asks the inside
; direction, which is the one an expansion-local rule would break.
;
; Invoked ONCE, and that is forced rather than sloppy: m18 measured a second
; `Al label $100` as `#1000 symbol double defined` EVEN WITH THE SAME VALUE, so
; there is no two-instance version of this shape that assembles. The
; discriminator is the VALUE instead — $100 is nowhere near the PC here, so
; "read the directive's value" and "read the program counter" are different
; bytes, and "did not resolve" is a third.
mlab	macro
Al	label	$100
	dc.w	Al
	endm
	mlab
	dc.w	Al
	dc.w	$4444
