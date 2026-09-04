	cpu	68000
	padding	off
	org	0
; --- enum member then X ---
	enum	Ar=5
Ar	set	2
	enum	Br=5
Br	equ	2
; --- string-valued equ then set ---
Cr	equ	"abc"
Cr	set	"def"
Dr	set	"abc"
Dr	equ	"def"
; --- float-valued equ then set ---
Er	equ	1.5
Er	set	2.5
Fr	set	1.5
Fr	equ	2.5
; --- a `set` re-run with the same value ---
Gr	set	1
Gr	set	1
	dc.w	$4444
