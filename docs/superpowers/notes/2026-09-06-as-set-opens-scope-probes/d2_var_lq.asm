; d2: reference the local under the `set` NAME.
;
; `Var.lq` exists only if `set` DID open a scope.  Same single occurrence of
; `lq`, so the two probes are exact opposites and one of them must fail on
; any assembler.
;
; MUST FAIL on an assembler that does not open a scope for `set`: the symbol
; simply does not exist there.  asl assembles it, exit 0, `00 00 10 02`.
	cpu	68000
	padding	off
	org	$1000
Parent:
	nop
Var	set	5
.lq:
	nop
	dc.l	Var.lq
	end
