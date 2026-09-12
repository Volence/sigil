; body `Inner:`; read `Inner` after the call (control: body label stays in the body)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
Inner:	dc.w	$2222
	endm
	mac
	dc.w	Inner	; REF
	dc.w	$4444
