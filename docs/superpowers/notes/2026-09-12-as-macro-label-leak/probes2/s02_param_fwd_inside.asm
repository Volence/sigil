; body `dc.w nm` then `nm:`, invoked with Foo and Bar
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	nm
	dc.w	nm	; REF
nm:	dc.w	$2222
	endm
	mac	Foo
	mac	Bar
	dc.w	$4444
