	cpu z80
	org 0
m	macro pa,pb
	message "(pa)(pb)"
	db ARGCOUNT
	endm
	m af',bb
	db 0EEh
	end
