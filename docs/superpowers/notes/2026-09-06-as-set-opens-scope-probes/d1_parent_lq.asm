; d1: reference the local under the PRECEDING LABEL's name.
;
; `Parent.lq` exists only if `set` opened NO scope.  `lq` is written once in
; this file, so the spelling that resolves names the parent outright; a name
; that existed under both parents could not.
;
; MUST FAIL on an assembler that matches asl: asl reports
; `#1010 symbol undefined` here and exits 2.
	cpu	68000
	padding	off
	org	$1000
Parent:
	nop
Var	set	5
.lq:
	nop
	dc.l	Parent.lq
	end
