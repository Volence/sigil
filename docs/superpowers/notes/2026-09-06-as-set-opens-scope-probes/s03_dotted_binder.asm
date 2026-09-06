; s03: does a DOTTED name in the label field of a value binder open a scope,
; and under what spelling?
;
; Discriminating names: `Outer.b.zz` and `Outer.zz` are different table rows,
; and `zz` appears exactly once in the source, so whichever row exists names
; the parent with no second reading.
	cpu	68000
	padding	off
	org	$1000
Outer:
	nop
.b	set	5
.zz:
	nop
Other:
	nop
.c	equ	6
.yy:
	nop
	end
