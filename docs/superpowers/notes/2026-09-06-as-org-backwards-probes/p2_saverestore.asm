; Does save/restore carry the program counter?
	cpu	68000
	org	$1000
	dc.b	1,2,3,4
	warning "P2A pc=\{*}h"
	save
	org	$10
	dc.b	$aa
	warning "P2B pc=\{*}h"
	restore
	warning "P2C pc=\{*}h"
	dc.b	$ee
	warning "P2D pc=\{*}h"
	end
