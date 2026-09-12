; `nextenum` member declared in a body after a file-level `enum`, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	enum	Ea=5
mac	macro
	nextenum	Eb
	endm
	mac
	dc.w	Eb	; REF
	dc.w	$4444
