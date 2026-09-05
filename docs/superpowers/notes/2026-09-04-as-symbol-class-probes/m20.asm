	cpu	68000
	padding	off
	org	0
; The MIXED ORDER the exemption has to get right, and the reason it must not
; merely skip the refusal but skip the RECORDING too: an expansion-local
; constant declared FIRST, then a file-level constant of the same name. asl has
; no such global symbol yet, so the file-level one is a FIRST declaration.
	rept	1
Pr:
	dc.w	$1111
	endm
Pr	equ	$99
	dc.w	Pr
menum	macro
	enum	Qe=5
	endm
	menum
Qe	equ	$99
	dc.w	Qe
; --- and the same order with the two forms that are NOT localized, which must
;     stay refusals ---
mlabdir	macro
Rl	label	$100
	endm
	mlabdir
Rl	equ	$99
mequ	macro
Sl	equ	7
	endm
	mequ
Sl	equ	$99
	dc.w	$4444
