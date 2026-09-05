	cpu	68000
	padding	off
	org	0
; --- the SAME header included twice ---
; The population the #1000 rule most plausibly has in a real source. Both the
; `equ` (identical value both times) and the label (two different addresses)
; are in it, so this separates "refuses on redefinition" from "refuses on value
; change" by itself.
	include	"m14hdr.inc"
	include	"m14hdr.inc"
	dc.w	$4444
