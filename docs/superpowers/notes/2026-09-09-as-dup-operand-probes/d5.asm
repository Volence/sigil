; d5 - a zero count. `$` before and after says whether it emits nothing.
	cpu	68000
	padding	off
	org	$1000
	dc.b	$11
	dc.b	[0]$FF
after	equ	*
	dc.b	after-$1000
	end
