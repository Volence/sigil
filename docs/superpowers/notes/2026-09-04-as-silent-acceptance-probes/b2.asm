	cpu	68000
	org	0
Bad:	equ	Undefined1+1
	if Bad=0
	dc.w	$1111
	else
	dc.w	$2222
	endif
	end
