	cpu	68000
	padding	off
	org	0
	dc.b	1!!0
	dc.b	5!!3
	dc.b	~~0
	dc.b	~~7
; PROBE lxor_vs_andand
	dc.b	1!!0&&0
	dc.b	(1!!0)&&0
	dc.b	1!!(0&&0)
; PROBE lxor_vs_oror
	dc.b	1!!1||1
	dc.b	(1!!1)||1
	dc.b	1!!(1||1)
; PROBE lxor_vs_eq
	dc.b	2=2!!0
	dc.b	(2=2)!!0
	dc.b	2=(2!!0)
