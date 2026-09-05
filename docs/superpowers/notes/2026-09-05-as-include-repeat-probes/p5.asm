	cpu	68000
	org	$1000
	include	"hdr_bytes.inc"
	include	"./hdr_bytes.inc"
	include	"sub/../hdr_bytes.inc"
	dc.b	$99
p5_end:
	dc.l	p5_end-$1000
