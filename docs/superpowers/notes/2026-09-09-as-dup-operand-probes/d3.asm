; d3 - the count as an expression, and as a symbol defined above the use.
n	equ	4
	cpu	68000
	padding	off
	org	$1000
	dc.b	[1+2]$AA
	dc.b	[(2*2)]$BB
	dc.b	[n]$CC
	dc.b	[n-1]$DD
	end
