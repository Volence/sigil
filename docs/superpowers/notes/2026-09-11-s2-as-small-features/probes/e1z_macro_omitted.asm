	cpu z80
	org 0
m macro a,b
	db (a)|(b)
	endm
	m 5
	db 0EEh
	end
