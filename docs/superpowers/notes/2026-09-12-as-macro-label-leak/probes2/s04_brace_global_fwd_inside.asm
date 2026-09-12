; file-level cnt; body `dc.w Lab{cnt}` then `Lab{cnt}:`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
cnt	=	7
mac	macro
	dc.w	Lab{cnt}	; REF
Lab{cnt}:	dc.w	$2222
	endm
	mac
	dc.w	$4444
