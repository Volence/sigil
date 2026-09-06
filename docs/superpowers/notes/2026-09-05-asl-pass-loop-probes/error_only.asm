; THE OTHER CONTROL, and the one that keeps the check off correct runs. One
; error, no forward reference anywhere, so asl never wanted a second pass.
; asl: 1 pass, 1 error, exit 2, footer WITHOUT the warning.
;
; Compare `error_first.asm`: also 1 pass, also 1 error, also exit 2. The two
; differ in nothing a runner sees except the one prose line in the listing
; footer, and the console summary is IDENTICAL for both. A detector keyed to
; "the run failed", or to "asl ran only one pass", cannot separate them.
	cpu	68000
	org	$1000
start:
	zzbogus	d0,d1
	rts
	end
