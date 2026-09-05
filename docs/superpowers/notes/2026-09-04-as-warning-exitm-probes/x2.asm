	cpu 68000
	padding off
	org 0
n := 42
s := "\{n}"
	dc.b s
	dc.b $FF
