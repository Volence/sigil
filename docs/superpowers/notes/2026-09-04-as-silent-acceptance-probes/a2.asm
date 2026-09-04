	cpu	68000
	org	0
Fwd:	equ	Later+1
	dc.w	Fwd
Later:	equ	$20
	dc.w	$4444
	end
