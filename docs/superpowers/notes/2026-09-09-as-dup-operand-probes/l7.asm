; l7 - `page` with THREE arguments, one past the upper end.
	cpu	68000
	org	$1000
	page	0,1,2
	dc.b	$11
	end
