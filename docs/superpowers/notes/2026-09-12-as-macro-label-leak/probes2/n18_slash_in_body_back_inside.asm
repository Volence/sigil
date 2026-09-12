; body `/` then `dc.w -` inside, invoked twice
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
/	dc.w	$2222
	dc.w	-	; REF
	endm
	mac
	mac
	dc.w	$4444
