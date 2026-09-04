	cpu	68000
	padding	off
	org	0
; --- set then X ---
As	set	1
As	equ	2
Bs	set	1
Bs	=	2
Cs	set	1
Cs	set	2
Ds	set	1
Ds	eval	2
Es	set	1
Es	:=	2
	dc.w	$4444
