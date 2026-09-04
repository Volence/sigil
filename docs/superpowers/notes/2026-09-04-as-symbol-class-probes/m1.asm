	cpu	68000
	padding	off
	org	0
; --- equ then X ---
Ae	equ	1
Ae	equ	2
Be	equ	1
Be	=	2
Ce	equ	1
Ce	set	2
De	equ	1
De	eval	2
Ee	equ	1
Ee	:=	2
	dc.w	$4444
