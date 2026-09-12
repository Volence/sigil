; `label *` in body, read after (control: global)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
Xl	label	*
	endm
	mac
	dc.w	Xl	; REF
	dc.w	$4444
