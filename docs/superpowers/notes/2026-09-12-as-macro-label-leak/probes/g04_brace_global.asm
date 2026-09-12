; body `Lab{cnt}:` with a file-level `cnt`, read `Lab7` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
cnt	=	7
mac	macro
Lab{cnt}:	dc.w	$2222
	endm
	mac
	dc.w	Lab7	; REF
	dc.w	$4444
