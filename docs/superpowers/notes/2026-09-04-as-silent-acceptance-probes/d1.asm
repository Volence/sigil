	cpu	68000
	org	0
	if Undefined1=0
	dc.w	$1111
	else
	dc.w	$2222
	endif
	end
