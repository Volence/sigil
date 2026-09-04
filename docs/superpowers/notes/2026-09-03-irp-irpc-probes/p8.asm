	cpu 68000
	padding off
	org $1000
; 8a: irpc over an integer expression
	dc.w $A501
	irpc c,65
	dc.b "<c>"
	endm
	dc.w $A502
	irpc c,1+2
	dc.b "<c>"
	endm
	dc.w $A503
; 8b: irp items are raw text, not evaluated
	irp v,1+2,$FF
	dc.b "[v]"
	endm
	dc.w $A504
; 8c: nesting a rept inside irpc, and irpc inside a macro with params
mm	macro pp,qq
	irpc c,"pp"
	dc.b "c",qq
	endm
	endm
	mm XY,7
	dc.w $A505
; 8d: shift inside an irp body
sh	macro aa
	irp v,1,2,3
	dc.b "aa",v
	shift
	endm
	dc.b "aa"
	endm
	sh p1,p2,p3
	dc.w $A506
; 8e: irpc terminated by endr
	irpc c,"QR"
	dc.b "{c}"
	endr
	dc.w $A507
	end
