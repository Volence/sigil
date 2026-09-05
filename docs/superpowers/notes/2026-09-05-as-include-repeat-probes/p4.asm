	cpu	68000
	org	$1000
	include	"p4b.inc"
	include	"p4c.inc"
	dc.b	$99
p4_end:
	dc.l	p4_end-$1000
