	cpu	68000
	padding	off
	org	0
; Q1 (spellings) inside the SAME expansion, all four PC-label spellings the
; `#1000` parcel found asl localizes, each read back from inside the body it is
; written in. The macro runs twice at different addresses, so a global binding
; that let the second expansion win would print the SECOND address on the first
; expansion's read lines.
;   colon label · colon-less column-0 label · label on a data line
msp	macro
Ca:
	dc.w	Ca
Cb
	dc.w	Cb
Cc:	dc.w	$1111
	dc.w	Cc
	endm
	msp
	msp
	dc.w	$4444
