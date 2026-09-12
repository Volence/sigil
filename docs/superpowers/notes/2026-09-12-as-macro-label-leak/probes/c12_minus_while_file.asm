; column-1 `-` inside a file-level `while` body, `-` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
cnt	set	0
	while	cnt<1
-	dc.w	$2222
cnt	set	cnt+1
	endm
	dc.w	-	; REF
	dc.w	$4444
