	cpu	68000
	padding	off
	org	0
; Which of these sets the local-label scope? Nothing here is refused, so the
; answer separates "a `set` opens a scope" from "a REFUSED label still does".
Scope1:
Av	set	1
.loc1	equ	7
; And now the same question with a REFUSED constant-making form in between.
Scope2:
Bv	set	1
Bv:
.loc2	equ	8
	dc.w	$4444
