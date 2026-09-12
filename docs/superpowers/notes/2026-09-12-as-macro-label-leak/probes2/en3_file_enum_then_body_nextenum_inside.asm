; file-level `enum`; body `nextenum Eb`, read inside
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	enum	Ea=5
mac	macro
	nextenum	Eb
	dc.w	Eb	; REF
	endm
	mac
	dc.w	$4444
