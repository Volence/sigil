	cpu	68000
	org	$1000
	include	"hdr_bytes.inc"
	include	"hdr_bytes.inc"
	dc.b	$99
p1_end:
	dc.l	p1_end-$1000
