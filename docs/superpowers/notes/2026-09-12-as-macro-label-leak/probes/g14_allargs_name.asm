; label name via ALLARGS (`ALLARGS:`), read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
ALLARGS:	dc.w	$2222
	endm
	mac	Foo
	dc.w	Foo	; REF
	dc.w	$4444
