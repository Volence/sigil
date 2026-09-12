; label name arrives as argument text (`nm:`), read `Foo` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	nm
nm:	dc.w	$2222
	endm
	mac	Foo
	dc.w	Foo	; REF
	dc.w	$4444
