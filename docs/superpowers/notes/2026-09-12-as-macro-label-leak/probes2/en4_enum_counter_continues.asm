; body `enum Ea=5`; after the call, file-level `nextenum Ez`, read Ez
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	enum	Ea=5
	endm
	mac
	nextenum	Ez
	dc.w	Ez	; REF
	dc.w	$4444
