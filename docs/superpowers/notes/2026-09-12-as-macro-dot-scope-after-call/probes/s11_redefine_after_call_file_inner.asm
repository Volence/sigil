; file `Inner:`/`.x:` first; Base; call; `.x:` again after the call (double definition?)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Inner:	dc.w	$7777
.x:	dc.w	$8888
Base:	dc.w	$5555
mac	macro
Inner:	dc.w	$2222
	endm
	mac
.x:	dc.w	$6666	; REF
	dc.w	$4444
