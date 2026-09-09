; d6 - a FORWARD-referenced count: the symbol is defined below its use.
	cpu	68000
	padding	off
	org	$1000
	dc.b	[fwd]$AA
	dc.b	$11
fwd	equ	3
	end
