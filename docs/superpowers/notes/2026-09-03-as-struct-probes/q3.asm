	cpu 68000
	org $1000
S struct DOTS
a:	ds.b 1
b:	ds.w 1
	endstruct
	dc.l S.a
	dc.l S.b
	dc.l S.len
inst:	S
after:
	dc.l inst
	dc.l inst.a
	dc.l inst.b
	dc.l after
