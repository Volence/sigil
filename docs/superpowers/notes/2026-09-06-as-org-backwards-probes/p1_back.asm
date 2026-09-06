; Backwards org after bytes have been emitted, 68000.
	cpu	68000
	org	$1000
start:	dc.b	1,2,3,4
	warning "P1A pc=\{*}h"
	org	$10
low:	dc.b	$aa,$bb
	warning "P1B pc=\{*}h low=\{low}h start=\{start}h"
	org	$2000
hi:	dc.b	$cc
	warning "P1C pc=\{*}h hi=\{hi}h"
	end
