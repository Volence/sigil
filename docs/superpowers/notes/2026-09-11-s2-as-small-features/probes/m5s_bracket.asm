	cpu 68000
	padding off
	org 0
m	macro pa,pb,pc
	message "(pa)(pb)(pc)[ALLARGS]"
	dc.b ARGCOUNT
	endm
	m [1,2],3
	dc.b $EE
	end
