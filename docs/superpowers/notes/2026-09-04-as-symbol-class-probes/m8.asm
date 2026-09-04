	cpu	68000
	padding	off
	org	0
; Run this one with `-D Dv=1` on the command line (see README.md) — the
; question is what CLASS a command-line define carries.
Dv	set	2
Dw	equ	2
	dc.w	$4444
