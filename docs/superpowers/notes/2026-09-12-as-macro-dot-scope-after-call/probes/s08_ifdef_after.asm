; `.x:` under Base; call; `ifdef .x` after picks a word
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
.x:	dc.w	$6666
mac	macro
Inner:	dc.w	$2222
	endm
	mac
	ifdef	.x	; REF
	dc.w	$AAAA
	else
	dc.w	$BBBB
	endif
	dc.w	$4444
