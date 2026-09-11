	cpu z80
	org 0
Ver := 2
	pushv ,Ver
Ver := 4
	db Ver
	popv ,Ver
	db Ver
	db 0EEh
	end
