; IFDEF of a body label, asked after the expansion
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
Lp:	dc.w	$2222
	endm
	mac
	ifdef	Lp	; REF
	dc.w	$AAAA
	else
	dc.w	$BBBB
	endif
	dc.w	$4444
