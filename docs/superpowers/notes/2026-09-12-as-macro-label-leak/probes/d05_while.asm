; plain label in a file-level `while` body, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
cnt	set	0
	while	cnt<1
Lw:	dc.w	$2222
cnt	set	cnt+1
	endm
	dc.w	Lw	; REF
	dc.w	$4444
