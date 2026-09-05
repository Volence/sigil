	cpu	68000
	org	$1000
Six:	equ	4+2
Foo:
	dc.l	val(substr("JmpTo_Foo", Six, 3))
	end
