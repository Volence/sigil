; d8 - AS's own `dcb.b` builtin, for comparison with `dc.b [n]v`.
	cpu	68000
	padding	off
	org	$1000
	dcb.b	3,$FF
	dcb.w	2,$1234
	end
