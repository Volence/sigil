	cpu 68000
	padding off
	org 0
twice	function x,x*2
add10	function x,x+10
	message "f42=\{twice(21)}"
	message "f255=\{add10(245)}"
	message "fneg=\{twice(-21)}"
	dc.b $11
	end
