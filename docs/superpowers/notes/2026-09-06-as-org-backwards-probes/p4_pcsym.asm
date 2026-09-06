; Which token is the program counter under each CPU.
	cpu	68000
	org	$1000
	dc.b	1,2,3,4
	warning "P4A star=\{*}h"
	save
	org	0
	cpu	z80
	warning "P4B star=\{*}h"
	nop
	warning "P4C dollar=\{$}h"
	nop
	warning "P4D star=\{*}h"
	restore
	warning "P4E star=\{*}h"
	end
