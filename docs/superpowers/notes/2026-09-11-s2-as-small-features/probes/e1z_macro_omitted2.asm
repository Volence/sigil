	cpu z80
	org 0
m macro pa,pb
	db (pa)|(pb)
	endm
	m 5
	db 0EEh
	end
