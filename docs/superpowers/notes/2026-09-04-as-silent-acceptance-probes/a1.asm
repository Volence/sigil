	cpu	68000
	org	0
Known:	equ	$10
Bad:	equ	Undefined1+1
	dc.w	Bad
	dc.w	Known
	dc.w	$4444
	end
