; 2026-09-04-as-enum-probes/q10.asm with one accepted value computed above the
; refused enum line. If row 13's "value folds to 0" is a declined operand's
; substitute, the reference build echoes $5A here instead of 0.
	cpu 68000
	padding off
	org 0
	dc.b $5A
	enum a=fw,b
	dc.b a,b
fw EQU 4
