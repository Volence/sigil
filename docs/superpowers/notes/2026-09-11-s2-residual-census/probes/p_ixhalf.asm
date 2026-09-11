	cpu z80undoc
	org 0
	ld	a,iyl
	adc	a,iyu
	ld	e,ixl
	ld	d,ixu
	ld	ixl,a
	ld	ixu,b
	end
