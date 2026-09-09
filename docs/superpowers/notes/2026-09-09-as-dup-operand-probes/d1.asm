; d1 - `[count]value` on each dc width, and whether the count advances `$`.
	cpu	68000
	padding	off
	org	$1000
	dc.b	[3]$FF
here_b	equ	*
	dc.b	$11
	dc.w	[2]$1234
here_w	equ	*
	dc.l	[2]$AABBCCDD
here_l	equ	*
	dc.b	here_b-$1000
	dc.b	here_w-$1000
	dc.b	here_l-$1000
	end
