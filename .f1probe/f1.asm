	cpu 68000
	padding off
; --- INT() basics
	dc.l INT(3.7)
	dc.l INT(3.2)
	dc.l INT(-3.7)
	dc.l INT(-3.2)
	dc.l INT(3.0)
	dc.l INT(-3.0)
	dc.l int(3.7)
	dc.l INT(7)
; --- rounding idiom
	dc.l INT(2.5+0.5)
	dc.l INT(-2.5+0.5)
	dc.l INT(-3.5+0.5)
; --- float reaching dc without INT
	dc.l 3.7
	dc.w 3.7
	dc.b 3.7
; --- integer division: is / float or truncating?
	dc.l 7/2
	dc.l INT(7/2)
	dc.l -7/2
	dc.l 1/3*3
; --- mixed
	dc.l INT(7.0/2)
	dc.l INT(1.0*3)
	end
