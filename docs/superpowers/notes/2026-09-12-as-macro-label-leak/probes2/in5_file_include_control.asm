; a file-level include's label, read after (control)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	include	"in5_inc.inc"
	dc.w	Linc	; REF
	dc.w	$4444
