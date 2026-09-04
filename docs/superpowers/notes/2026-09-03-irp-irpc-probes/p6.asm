	cpu 68000
	padding off
	org $1000
; 6a: empty irp list
	dc.w $A301
	irp v,
	dc.b $11
	endm
	dc.w $A302
; 6b: irp with one empty-looking group
	irp v,,
	dc.b $22
	endm
	dc.w $A303
; 6c: irpc on empty string
	irpc c,""
	dc.b $33
	endm
	dc.w $A304
; 6d: irpc on a string SYMBOL (not a literal)
sstr	set "PQ"
	irpc c,sstr
	dc.b "c"
	endm
	dc.w $A305
; 6e: irpc on a bare identifier that is not a symbol
	irpc c,XY
	dc.b "c"
	endm
	dc.w $A306
; 6f: whitespace around irp items
	irp v, 11 , 22
	dc.b v
	endm
	dc.w $A307
; 6g: substitution boundary — inside quotes, single quotes, glued
	irpc c,"AB"
	dc.b "c", 'c', "xcx", "_c_"
	endm
	dc.w $A308
; 6h: irpc over a string with spaces
	irpc c,"P Q"
	dc.b "[c]"
	endm
	dc.w $A309
; 6i: nested irp inside irpc
	irpc c,"AB"
	irp v,1,2
	dc.b "c",v
	endm
	endm
	dc.w $A30A
	end
