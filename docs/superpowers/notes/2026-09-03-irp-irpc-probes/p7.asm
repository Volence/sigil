	cpu 68000
	padding off
	org $1000
; 7a: irpc on "" — how many iterations, and what is c?
	dc.w $A401
	irpc c,""
	dc.b "<c>"
	endm
	dc.w $A402
; 7b: irpc with a totally empty operand
	irpc c,
	dc.b "<c>"
	endm
	dc.w $A403
; 7c: irp with quoted comma inside an item
	irp v,"a,b","c"
	dc.b v
	endm
	dc.w $A404
; 7d: irpc with escapes in the string
	irpc c,"A\x5AB"
	dc.b "[c]"
	endm
	dc.w $A405
; 7e: irpc over a string built by concatenation
sa	set "MN"
	irpc c,sa+"OP"
	dc.b "(c)"
	endm
	dc.w $A406
; 7f: loop var case sensitivity under -U
	irpc Cv,"AB"
	dc.b "<Cv><cv>"
	endm
	dc.w $A407
; 7g: does the loop var leak as a symbol after the loop?
	irpc c,"Z"
	dc.b "c"
	endm
	dc.b "c"
	dc.w $A408
	end
