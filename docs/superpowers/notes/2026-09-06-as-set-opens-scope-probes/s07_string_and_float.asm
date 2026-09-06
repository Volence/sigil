; s07: the non-integer binder shapes.  A string `set`, a string `equ` and a
; float `set` all bind a symbol; do they all open a scope?
;
; Each local name occurs once, and none of `Sv`, `Se2`, `Fv` is a name any
; other line could give a local to.
	cpu	68000
	padding	off
	org	$1000
Anchor:
	nop
Sv	set	"abc"
.s1:
	nop
Se2	equ	"def"
.s2:
	nop
Fv	set	1.5
.s3:
	nop
	end
