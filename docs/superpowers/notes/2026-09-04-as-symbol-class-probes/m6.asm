	cpu	68000
	padding	off
	org	0
; --- a `set` inside a macro over a name the caller made a constant ---
mset	macro	n
n	set	9
	endm
mequ	macro	n
n	equ	9
	endm
Am	equ	1
	mset	Am
Bm	set	1
	mequ	Bm
; --- a `set` inside a REPT: the same line runs twice ---
	rept	2
Cm	set	Cm+1
	endm
; --- a LOCAL name under two scopes ---
Sc1:
.loc	equ	1
.loc	set	2
Sc2:
.loc	set	3
	dc.w	$4444
