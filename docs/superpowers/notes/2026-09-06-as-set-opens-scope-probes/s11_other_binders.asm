; s11: the binding forms the matrix does not cover, asked the same way.
; `enum` members and the `struct` field forms also put a name in the symbol
; table; do they open a scope too?
;
; Each local name occurs once and every candidate parent is distinct.
	cpu	68000
	padding	off
	org	$1000
Anchor:
	nop
	enum	En1,En2
.e1:
	nop
	end
