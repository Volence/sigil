	cpu 68000
	padding off
	supmode on
	org 0
	move.w	d0,Sym
Here:
	if Here < 6
	fatal "layout moved between passes"
	endif
	dc.w	Here
Sym	equ	$123456
	end
