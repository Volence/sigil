	cpu	68000
	padding	off
	org	0
; m18 showed an `enum` member declared inside a macro expansion is silent on a
; second expansion, and that its name is ABSENT from asl's symbol table — the
; same two signatures the localized PC labels carry. This confirms the reading
; the exemption is drawn on: `Be` must be UNRESOLVABLE outside the expansion,
; while `Al` (the `label` directive, which m18 showed IS `#1000` on the second
; expansion) must resolve.
menum	macro
	enum	Be=5
	endm
	menum
mlabdir	macro
Al	label	$100
	endm
	mlabdir
	dc.w	Al		; global — must resolve to $100
	dc.w	Be		; expansion-local — must be #1010
	dc.w	$4444
