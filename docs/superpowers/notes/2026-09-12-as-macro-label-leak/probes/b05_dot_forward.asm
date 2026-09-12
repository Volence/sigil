; `.dl:` in body under `Base:`, read BEFORE the expansion
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
.dl:	dc.w	$2222
	endm
	dc.w	.dl	; REF
	mac
	dc.w	$4444
