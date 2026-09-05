	cpu	68000
	padding	off
	phase	0
; A CONSTANT-valued `label` in a macro body, reached by a fixup the front end
; must DEFER. `p11`/`p12` and the m18/m19 pair all read their name with a
; backward `dc.w`, which folds in-pass out of the symbol environment — so none
; of them can see whether the name was ALSO placed as a relocatable label at the
; expansion's address. A `bra.w` is resolved from the section's symbol table
; instead, and the two readings give different displacements: `$100` is the
; directive's value, `$4` is where the expansion sits.
mk	macro
Al	label	$100
	nop
	endm
	bra.w	Al
	mk
	dc.w	Al
