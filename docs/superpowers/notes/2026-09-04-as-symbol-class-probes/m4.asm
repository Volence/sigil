	cpu	68000
	padding	off
	org	0
; --- same VALUE redefinition: is the refusal about class or about change? ---
Aq	equ	1
Aq	equ	1
Bq	equ	1
Bq	set	1
Cq	set	1
Cq	equ	1
; --- bare (colon-less, column 0) label then X ---
Dq
Dq	equ	2
	dc.w	$1111
Eq
Eq	set	2
	dc.w	$1111
; --- `label` directive then X ---
Fq	label	$100
Fq	equ	2
Gq	label	$100
Gq	set	2
; --- label on a data line, then X ---
Hq:	dc.w	$1111
Hq	set	2
	dc.w	$4444
