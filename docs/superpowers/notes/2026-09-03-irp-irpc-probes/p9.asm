	cpu 68000
	padding off
	org $1000
; 9a: ARGCOUNT inside a string, and boundary behaviour
ac2	macro pp
	dc.b "1[ARGCOUNT] 2[xARGCOUNTx] 3[_ARGCOUNT_] 4[argcount] 5[ArgCount]"
	endm
	dc.w $A601
	ac2 7,8
	dc.w $A602
; 9b: a parameter literally named ARGCOUNT
ac3	macro ARGCOUNT
	dc.b "[ARGCOUNT]"
	endm
	ac3 zz
	dc.w $A603
; 9c: irp with no comma at all
	irp v
	dc.b $99
	endm
	dc.w $A604
; 9d: irpc with no comma at all
	irpc c
	dc.b $98
	endm
	dc.w $A605
; 9e: irp closed by endr
	irp v,4,5
	dc.b v
	endr
	dc.w $A606
	end
